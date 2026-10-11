use ibapi_integration_async::connect;
use ibapi_test::ClientId;

#[tokio::test]
async fn is_connected_after_connect() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;
    assert!(client.is_connected());
}

#[tokio::test]
async fn client_id_matches() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;
    assert_eq!(client.client_id(), client_id.id());
}

#[tokio::test]
async fn next_order_id_is_positive() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;
    assert!(client.next_order_id() > 0);
}

#[tokio::test]
async fn next_order_id_increments() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;
    let first = client.next_order_id();
    let second = client.next_order_id();
    assert!(second > first);
}
