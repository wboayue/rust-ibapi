use ibapi_integration_sync::connect;
use ibapi_test::ClientId;

#[test]
#[ignore]
fn wsh_metadata() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let metadata = client.wsh_metadata().expect("wsh_metadata failed");
    assert!(!metadata.data_json.is_empty());
}

#[test]
#[ignore]
fn wsh_event_data_by_contract() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    // AAPL contract_id (commonly 265598)
    let data = client
        .wsh_event_data_by_contract(265598)
        .fetch()
        .expect("wsh_event_data_by_contract failed");
    assert!(!data.data_json.is_empty());
}

#[test]
#[ignore]
fn wsh_event_data_by_filter() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let subscription = client
        .wsh_event_data_by_filter("{}")
        .subscribe()
        .expect("wsh_event_data_by_filter failed");
    let _item = subscription.next();
}
