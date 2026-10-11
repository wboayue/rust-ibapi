use ibapi_integration_sync::connect;
use ibapi_test::ClientId;

#[test]
fn is_connected_after_connect() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
    assert!(client.is_connected());
}

#[test]
fn client_id_matches() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
    assert_eq!(client.client_id(), client_id.id());
}

#[test]
fn next_order_id_is_positive() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
    assert!(client.next_order_id() > 0);
}

#[test]
fn next_order_id_increments() {
    let client_id = ClientId::get();
    let client = connect(&client_id);
    let first = client.next_order_id();
    let second = client.next_order_id();
    assert!(second > first);
}
