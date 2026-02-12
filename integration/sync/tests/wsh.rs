use ibapi::client::blocking::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
#[ignore]
fn wsh_metadata() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let metadata = client.wsh_metadata().expect("wsh_metadata failed");
    assert!(!metadata.data_json.is_empty());
}

#[test]
#[ignore]
fn wsh_event_data_by_contract() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    // AAPL contract_id (commonly 265598)
    rate_limit();
    let data = client
        .wsh_event_data_by_contract(265598, None, None, None, None)
        .expect("wsh_event_data_by_contract failed");
    assert!(!data.data_json.is_empty());
}

#[test]
#[ignore]
fn wsh_event_data_by_filter() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let subscription = client
        .wsh_event_data_by_filter("{}", None, None)
        .expect("wsh_event_data_by_filter failed");
    let _item = subscription.next();
}
