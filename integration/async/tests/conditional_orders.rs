use ibapi::contracts::Contract;
use ibapi::orders::conditions::{
    ExecutionCondition, MarginCondition, PercentChangeCondition, PriceCondition, TimeCondition, TriggerMethod, VolumeCondition,
};
use ibapi::orders::{Action, Order, OrderCondition, PlaceOrder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Client, Error};
use ibapi_test::{condition_time_today, rate_limit, ClientId, AAPL_CON_ID, GATEWAY, SPY_CON_ID, TSLA_CON_ID};
use serial_test::serial;
use tokio::time::{timeout, Duration};

async fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    (client, client_id)
}

async fn submit_and_cleanup(client: &Client, contract: &Contract, order: &Order) {
    rate_limit();
    let order_id = client.next_order_id();
    let mut subscription = client.place_order(order_id, contract, order).await.expect("place_order failed");

    let mut acknowledged = false;
    while let Ok(Some(result)) = timeout(Duration::from_secs(5), subscription.next()).await {
        match result {
            Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(_) | PlaceOrder::OpenOrder(_))) => {
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Notice(notice)) => {
                if notice.message.contains("rejected") {
                    panic!("TWS rejected conditional order: {}", notice.message);
                }
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Data(_)) => continue,
            Err(Error::Message(201, msg)) => panic!("TWS rejected conditional order [201]: {msg}"),
            Err(Error::Message(_, _)) => {
                acknowledged = true;
                break;
            }
            Err(e) => panic!("subscription error: {e}"),
        }
    }
    assert!(acknowledged, "no acknowledgement from TWS within timeout");

    rate_limit();
    let _ = client.cancel_order(order_id, "").await;
}

// Use a far-from-market limit so the order never fills even if a condition
// triggers between submission and cleanup-cancel.
fn conditional_limit_order(action: Action, quantity: f64, conditions: Vec<OrderCondition>) -> Order {
    let limit_price = match action {
        Action::Buy => 1.0,
        _ => 9999.0,
    };
    Order {
        action,
        total_quantity: quantity,
        order_type: "LMT".to_string(),
        limit_price: Some(limit_price),
        conditions,
        ..Default::default()
    }
}

#[tokio::test]
#[serial(orders)]
async fn price_condition() {
    let (client, _client_id) = connect().await;
    let condition = PriceCondition::builder(AAPL_CON_ID, "SMART")
        .greater_than(200.0)
        .trigger_method(TriggerMethod::Default)
        .build();

    let contract = Contract::stock("MSFT").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Price(condition)]);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn time_condition() {
    let (client, _client_id) = connect().await;
    let condition = TimeCondition::builder().greater_than(condition_time_today(14, 30)).build();

    let contract = Contract::stock("AAPL").build();
    let mut order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Time(condition)]);
    order.conditions_ignore_rth = true;
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn margin_condition() {
    let (client, _client_id) = connect().await;
    let condition = MarginCondition::builder().less_than(30).build();

    let contract = Contract::stock("TSLA").build();
    let mut order = conditional_limit_order(Action::Sell, 5.0, vec![OrderCondition::Margin(condition)]);
    order.conditions_cancel_order = true;
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn execution_condition() {
    let (client, _client_id) = connect().await;
    let condition = ExecutionCondition::builder("MSFT", "STK", "SMART").build();

    let contract = Contract::stock("AAPL").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Execution(condition)]);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn volume_condition() {
    let (client, _client_id) = connect().await;
    let condition = VolumeCondition::builder(TSLA_CON_ID, "SMART").greater_than(50_000_000).build();

    let contract = Contract::stock("TSLA").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Volume(condition)]);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn percent_change_condition() {
    let (client, _client_id) = connect().await;
    let condition = PercentChangeCondition::builder(SPY_CON_ID, "SMART").greater_than(2.0).build();

    let contract = Contract::stock("SPY").build();
    let order = conditional_limit_order(Action::Sell, 10.0, vec![OrderCondition::PercentChange(condition)]);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn multiple_conditions_with_and() {
    let (client, _client_id) = connect().await;

    let price = PriceCondition::builder(AAPL_CON_ID, "SMART")
        .greater_than(180.0)
        .conjunction(true)
        .build();
    let time = TimeCondition::builder()
        .greater_than(condition_time_today(15, 0))
        .conjunction(true)
        .build();

    let contract = Contract::stock("AAPL").build();
    let mut order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Price(price), OrderCondition::Time(time)]);
    order.conditions_ignore_rth = false;
    submit_and_cleanup(&client, &contract, &order).await;
}
