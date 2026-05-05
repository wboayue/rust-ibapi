use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::conditions::{
    ExecutionCondition, MarginCondition, PercentChangeCondition, PriceCondition, TimeCondition, TriggerMethod, VolumeCondition,
};
use ibapi::orders::{Action, Order, OrderCondition, PlaceOrder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::Error;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;
use time::OffsetDateTime;

const AAPL_CON_ID: i32 = 265598;
const TSLA_CON_ID: i32 = 76792991;
const SPY_CON_ID: i32 = 756733;

fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    (client, client_id)
}

fn future_time(hour: u8, minute: u8) -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02} {:02}:{:02}:00 US/Eastern",
        now.year(),
        now.month() as u8,
        now.day(),
        hour,
        minute
    )
}

/// Submits the order, waits for TWS to acknowledge it (OrderStatus/OpenOrder, a
/// non-fatal warning, or a non-rejection Notice), then cancels. Hard rejections
/// (TWS code 201) fail the test.
fn submit_and_cleanup(client: &Client, contract: &Contract, order: &Order) {
    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, contract, order).expect("place_order failed");

    let mut acknowledged = false;
    while let Some(result) = subscription.next_timeout(Duration::from_secs(5)) {
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
            // Code 201 = hard rejection. Other Error::Message values (399 after-hours
            // queueing, 2174 timezone warning, etc.) are non-fatal — TWS *accepted*
            // the order and the dispatcher just terminates the subscription on them.
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
    let _ = client.cancel_order(order_id, "");
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

#[test]
#[serial(orders)]
fn price_condition() {
    let (client, _client_id) = connect();
    let condition = PriceCondition::builder(AAPL_CON_ID, "SMART")
        .greater_than(200.0)
        .trigger_method(TriggerMethod::Default)
        .build();

    let contract = Contract::stock("MSFT").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Price(condition)]);
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn time_condition() {
    let (client, _client_id) = connect();
    let condition = TimeCondition::builder().greater_than(future_time(14, 30)).build();

    let contract = Contract::stock("AAPL").build();
    let mut order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Time(condition)]);
    order.conditions_ignore_rth = true;
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn margin_condition() {
    let (client, _client_id) = connect();
    let condition = MarginCondition::builder().less_than(30).build();

    let contract = Contract::stock("TSLA").build();
    let mut order = conditional_limit_order(Action::Sell, 5.0, vec![OrderCondition::Margin(condition)]);
    order.conditions_cancel_order = true;
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn execution_condition() {
    let (client, _client_id) = connect();
    let condition = ExecutionCondition::builder("MSFT", "STK", "SMART").build();

    let contract = Contract::stock("AAPL").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Execution(condition)]);
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn volume_condition() {
    let (client, _client_id) = connect();
    let condition = VolumeCondition::builder(TSLA_CON_ID, "SMART").greater_than(50_000_000).build();

    let contract = Contract::stock("TSLA").build();
    let order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Volume(condition)]);
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn percent_change_condition() {
    let (client, _client_id) = connect();
    let condition = PercentChangeCondition::builder(SPY_CON_ID, "SMART").greater_than(2.0).build();

    let contract = Contract::stock("SPY").build();
    let order = conditional_limit_order(Action::Sell, 10.0, vec![OrderCondition::PercentChange(condition)]);
    submit_and_cleanup(&client, &contract, &order);
}

#[test]
#[serial(orders)]
fn multiple_conditions_with_and() {
    let (client, _client_id) = connect();

    let price = PriceCondition::builder(AAPL_CON_ID, "SMART")
        .greater_than(180.0)
        .conjunction(true)
        .build();
    let time = TimeCondition::builder().greater_than(future_time(15, 0)).conjunction(true).build();

    let contract = Contract::stock("AAPL").build();
    let mut order = conditional_limit_order(Action::Buy, 10.0, vec![OrderCondition::Price(price), OrderCondition::Time(time)]);
    order.conditions_ignore_rth = false;
    submit_and_cleanup(&client, &contract, &order);
}
