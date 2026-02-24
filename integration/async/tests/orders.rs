use ibapi::contracts::Contract;
use ibapi::orders::{Action, CancelOrder, ExecutionFilter, Order};
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
    client.cancel_order(order_id, "").await.expect("cancel_order failed");
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
    client.cancel_order(order_id, "").await.expect("cancel_order failed");
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
    client.cancel_order(order_id.0, "").await.expect("cancel_order failed");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[tokio::test]
#[serial(orders)]
async fn cancel_bracket_order() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let ids = client
        .order(&contract)
        .buy(1)
        .bracket()
        .entry_limit(1.0)
        .take_profit(300.0)
        .stop_loss(0.50)
        .submit_all()
        .await
        .expect("bracket order submit failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Cancel the parent order — child orders should be cancelled automatically
    rate_limit();
    let mut cancel_sub = client.cancel_order(ids.parent.0, "").await.expect("cancel_order failed");

    let item = tokio::time::timeout(tokio::time::Duration::from_secs(10), cancel_sub.next()).await;
    assert!(item.is_ok(), "cancel order status timed out");
    let status = item.unwrap().expect("expected cancel order status").expect("cancel order returned error");
    match status {
        CancelOrder::OrderStatus(s) => assert_eq!(s.status, "Cancelled", "parent order should be cancelled"),
        CancelOrder::Notice(_) => {} // cancellation notice is also acceptable
    }
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[tokio::test]
#[serial(orders)]
async fn cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();

    // Place bracket order
    rate_limit();
    let ids = client
        .order(&contract)
        .buy(1)
        .bracket()
        .entry_limit(1.0)
        .take_profit(300.0)
        .stop_loss(0.50)
        .submit_all()
        .await
        .expect("bracket order submit failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Cancel the bracket order
    rate_limit();
    let _cancel_sub = client.cancel_order(ids.parent.0, "").await.expect("cancel_order failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Place a new order after cancellation — should succeed without errors
    rate_limit();
    let new_order_id = client
        .order(&contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .await
        .expect("placing order after cancel should succeed");

    assert!(new_order_id.0 > 0, "new order id should be positive");

    // Cleanup
    rate_limit();
    client.cancel_order(new_order_id.0, "").await.expect("cleanup cancel failed");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[tokio::test]
#[serial(orders)]
async fn global_cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect().await;

    let contract = Contract::stock("AAPL").build();

    // Place bracket order
    rate_limit();
    let _ids = client
        .order(&contract)
        .buy(1)
        .bracket()
        .entry_limit(1.0)
        .take_profit(300.0)
        .stop_loss(0.50)
        .submit_all()
        .await
        .expect("bracket order submit failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Global cancel all orders
    rate_limit();
    client.global_cancel().await.expect("global_cancel failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Place a new order after global cancel — should succeed without errors
    rate_limit();
    let new_order_id = client
        .order(&contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .await
        .expect("placing order after global cancel should succeed");

    assert!(new_order_id.0 > 0, "new order id should be positive");

    // Cleanup
    rate_limit();
    client.cancel_order(new_order_id.0, "").await.expect("cleanup cancel failed");
}

#[tokio::test]
#[serial(orders)]
async fn executions_returns_subscription() {
    let (client, _client_id) = connect().await;

    rate_limit();
    let mut subscription = client.executions(ExecutionFilter::default()).await.expect("executions failed");
    let _item = tokio::time::timeout(tokio::time::Duration::from_secs(5), subscription.next()).await;
}
