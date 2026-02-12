use ibapi::client::blocking::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
fn is_connected_after_connect() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    assert!(client.is_connected());
}

#[test]
fn client_id_matches() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    assert_eq!(client.client_id(), client_id.id());
}

#[test]
fn next_order_id_is_positive() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    assert!(client.next_order_id() > 0);
}

#[test]
fn next_order_id_increments() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    let first = client.next_order_id();
    let second = client.next_order_id();
    assert!(second > first);
}

#[test]
fn next_request_id_is_positive() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");
    assert!(client.next_request_id() > 0);
}
