use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::{Action, ExecutionFilter, Order};
use ibapi_test::{rate_limit, ClientId, GATEWAY};
use serial_test::serial;

fn connect() -> Client {
    let client_id = ClientId::get();
    rate_limit();
    Client::connect(GATEWAY, client_id.id()).expect("connection failed")
}

fn limit_order(action: Action, quantity: f64, price: f64) -> Order {
    let mut order = Order::default();
    order.action = action;
    order.total_quantity = quantity;
    order.order_type = "LMT".to_string();
    order.limit_price = Some(price);
    order
}

#[test]
#[serial(orders)]
fn next_valid_order_id() {
    let client = connect();

    rate_limit();
    let id = client.next_valid_order_id().expect("next_valid_order_id failed");
    assert!(id > 0, "order id should be positive");
}

#[test]
#[serial(orders)]
fn open_orders() {
    let client = connect();

    rate_limit();
    let subscription = client.open_orders().expect("open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn all_open_orders() {
    let client = connect();

    rate_limit();
    let subscription = client.all_open_orders().expect("all_open_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders() {
    let client = connect();

    rate_limit();
    let subscription = client.completed_orders(false).expect("completed_orders failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn completed_orders_api_only() {
    let client = connect();

    rate_limit();
    let subscription = client.completed_orders(true).expect("completed_orders api_only failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}

#[test]
#[serial(orders)]
fn place_limit_buy() {
    let client = connect();

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
    let _cancel = client.cancel_order(order_id, "");
}

#[test]
#[serial(orders)]
fn place_limit_sell() {
    let client = connect();

    let contract = Contract::stock("AAPL").build();
    let order = limit_order(Action::Sell, 1.0, 9999.0); // Far above market

    rate_limit();
    let order_id = client.next_order_id();
    let subscription = client.place_order(order_id, &contract, &order).expect("place_order failed");

    let item = subscription.next_timeout(Duration::from_secs(10));
    assert!(item.is_some(), "expected order status update");

    rate_limit();
    let _cancel = client.cancel_order(order_id, "");
}

#[test]
#[serial(orders)]
fn cancel_order_succeeds() {
    let client = connect();

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
    let client = connect();

    rate_limit();
    client.global_cancel().expect("global_cancel failed");
}

#[test]
#[serial(orders)]
fn order_builder_limit() {
    let client = connect();

    let contract = Contract::stock("AAPL").build();

    rate_limit();
    let order_id = client.order(&contract).buy(1).limit(1.0).submit().expect("order builder submit failed");

    assert!(order_id.0 > 0, "order id should be positive");

    // Cancel the placed order
    rate_limit();
    let _cancel = client.cancel_order(order_id.0, "");
}

#[test]
#[serial(orders)]
fn executions_returns_subscription() {
    let client = connect();

    rate_limit();
    let subscription = client.executions(ExecutionFilter::default()).expect("executions failed");
    let _item = subscription.next_timeout(Duration::from_secs(5));
}
