use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
async fn is_connected_after_connect() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    assert!(client.is_connected());
}

#[tokio::test]
async fn client_id_matches() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    assert_eq!(client.client_id(), client_id.id());
}

#[tokio::test]
async fn next_order_id_is_positive() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    assert!(client.next_order_id() > 0);
}

#[tokio::test]
async fn next_order_id_increments() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    let first = client.next_order_id();
    let second = client.next_order_id();
    assert!(second > first);
}

#[tokio::test]
async fn next_request_id_is_positive() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    assert!(client.next_request_id() > 0);
}
