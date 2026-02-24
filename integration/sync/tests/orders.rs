use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{Action, CancelOrder, ExecutionFilter, Order};
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
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

#[test]
#[serial(orders)]
fn next_valid_order_id() {
    let (client, _client_id) = connect();

    rate_limit();
    let id = client.next_valid_order_id().expect("next_valid_order_id failed");
    assert!(id > 0, "order id should be positive");
}

#[test]
#[serial(orders)]
fn open_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.open_orders().expect("open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn all_open_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.all_open_orders().expect("all_open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.completed_orders(false).expect("completed_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders_api_only() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.completed_orders(true).expect("completed_orders api_only failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn place_limit_buy() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0); // Far below market

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    // Should receive order status
    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected order status update");

    // Cancel the order
    rate_limit();
    client.cancel_order(order_id, "").expect("cancel_order failed");
}

#[test]
#[serial(orders)]
fn place_limit_sell() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Sell, 1.0, 9999.0); // Far above market

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected order status update");

    rate_limit();
    client.cancel_order(order_id, "").expect("cancel_order failed");
}

#[test]
#[serial(orders)]
fn cancel_order_succeeds() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Buy, 1.0, 1.0);

    rate_limit();
    let order_id = client.next_order_id();
    let _subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    std::thread::sleep(Duration::from_millis(500));

    rate_limit();
    let cancel_sub = client.cancel_order(order_id, "").expect("cancel_order failed");
    let _item = cancel_sub.next_timeout(Duration::from_secs(10));
}

#[test]
#[serial(orders)]
fn global_cancel() {
    let (client, _client_id) = connect();

    rate_limit();
    client.global_cancel().expect("global_cancel failed");
}

#[test]
#[serial(orders)]
fn order_builder_limit() {
    let (client, _client_id) = connect();

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let order_id = client.order(&contract).buy(1).limit(1.0).submit().expect("order builder submit failed");

    assert!(order_id.0 > 0, "order id should be positive");

    // Cancel the placed order
    rate_limit();
    client.cancel_order(order_id.0, "").expect("cancel_order failed");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn cancel_bracket_order() {
    let (client, _client_id) = connect();

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
        .expect("bracket order submit failed");

    std::thread::sleep(Duration::from_millis(500));

    // Cancel the parent order — child orders should be cancelled automatically
    rate_limit();
    let cancel_sub = client.cancel_order(ids.parent.0, "").expect("cancel_order failed");

    let item = cancel_sub.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "cancel order status timed out");
    match item.unwrap() {
        CancelOrder::OrderStatus(s) => assert_eq!(s.status, "Cancelled", "parent order should be cancelled"),
        CancelOrder::Notice(_) => {} // cancellation notice is also acceptable
    }
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect();

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
        .expect("bracket order submit failed");

    std::thread::sleep(Duration::from_millis(500));

    // Cancel the bracket order
    rate_limit();
    let _cancel_sub = client.cancel_order(ids.parent.0, "").expect("cancel_order failed");

    std::thread::sleep(Duration::from_millis(500));

    // Place a new order after cancellation — should succeed without errors
    rate_limit();
    let new_order_id = client
        .order(&contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .expect("placing order after cancel should succeed");

    assert!(new_order_id.0 > 0, "new order id should be positive");

    // Cleanup
    rate_limit();
    client.cancel_order(new_order_id.0, "").expect("cleanup cancel failed");
}

// Regression test for https://github.com/wboayue/rust-ibapi/issues/426
#[test]
#[serial(orders)]
fn global_cancel_bracket_order_then_place_new_order() {
    let (client, _client_id) = connect();

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
        .expect("bracket order submit failed");

    std::thread::sleep(Duration::from_millis(500));

    // Global cancel all orders
    rate_limit();
    client.global_cancel().expect("global_cancel failed");

    std::thread::sleep(Duration::from_millis(500));

    // Place a new order after global cancel — should succeed without errors
    rate_limit();
    let new_order_id = client
        .order(&contract)
        .buy(1)
        .limit(1.0)
        .submit()
        .expect("placing order after global cancel should succeed");

    assert!(new_order_id.0 > 0, "new order id should be positive");

    // Cleanup
    rate_limit();
    client.cancel_order(new_order_id.0, "").expect("cleanup cancel failed");
}

#[test]
#[serial(orders)]
fn executions_returns_subscription() {
    let (client, _client_id) = connect();

    rate_limit();
    let subscription = client.executions(ExecutionFilter::default()).expect("executions failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}
