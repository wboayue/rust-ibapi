use ibapi::contracts::Contract;
use ibapi::orders::{Action, ExecutionFilter, Order};
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

async fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    (client, client_id)
}

fn limit_order(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        total_quantity: quantity,
        order_type: "LMT".to_string(),
        limit_price: Some(price),
        ..Default::default()
    }
}

#[tokio::test]
#[serial(orders)]
async fn next_valid_order_id() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let id = client.next_valid_order_id().await.expect("next_valid_order_id failed");
    assert!(id > 0, "order id should be positive");
}

#[tokio::test]
#[serial(orders)]
async fn open_orders() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.open_orders().await.expect("open_orders failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(orders)]
async fn all_open_orders() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.all_open_orders().await.expect("all_open_orders failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(orders)]
async fn completed_orders() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.completed_orders(false).await.expect("completed_orders failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(orders)]
async fn completed_orders_api_only() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.completed_orders(true).await.expect("completed_orders api_only failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}

#[tokio::test]
#[serial(orders)]
async fn place_limit_buy() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0);

    rate_limit();
    let order_id = client.next_order_id();
    let mut subscription = client.place_order(order_id, &contract, &order).await.expect("place_order failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "order status timed out");
    assert!(item.unwrap().is_some(), "expected order status update");

    rate_limit();
    let _cancel = client.cancel_order(order_id, "").await;
}

#[tokio::test]
#[serial(orders)]
async fn place_limit_sell() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Sell, 1.0, 9999.0);

    rate_limit();
    let order_id = client.next_order_id();
    let mut subscription = client.place_order(order_id, &contract, &order).await.expect("place_order failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), subscription.next()).await;
    assert!(item.is_ok(), "order status timed out");
    assert!(item.unwrap().is_some(), "expected order status update");

    rate_limit();
    let _cancel = client.cancel_order(order_id, "").await;
}

#[tokio::test]
#[serial(orders)]
async fn cancel_order_succeeds() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0);

    rate_limit();
    let order_id = client.next_order_id();
    let _subscription = client.place_order(order_id, &contract, &order).await.expect("place_order failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    rate_limit();
    let mut cancel_sub = client.cancel_order(order_id, "").await.expect("cancel_order failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(10), cancel_sub.next()).await;
}

#[tokio::test]
#[serial(orders)]
async fn global_cancel() {
    let (client, _client_id) = connect().await;

    rate_limit();
    client.global_cancel().await.expect("global_cancel failed");
}

#[tokio::test]
#[serial(orders)]
async fn order_builder_limit() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let order_id = client
        .order(&contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .await
        .expect("order builder submit failed");

    assert!(order_id.0 > 0, "order id should be positive");

    // Cancel the placed order
    rate_limit();
    let _cancel = client.cancel_order(order_id.0, "").await;
}

#[tokio::test]
#[serial(orders)]
async fn executions_returns_subscription() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.executions(ExecutionFilter::default()).await.expect("executions failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}
