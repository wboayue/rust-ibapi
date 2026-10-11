use futures::StreamExt;
use ibapi_integration_async::connect;
use ibapi_test::ClientId;

#[tokio::test]
#[ignore]
async fn wsh_metadata() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let metadata = client.wsh_metadata().await.expect("wsh_metadata failed");
    assert!(!metadata.data_json.is_empty());
}

#[tokio::test]
#[ignore]
async fn wsh_event_data_by_contract() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    // AAPL contract_id (commonly 265598)
    let data = client
        .wsh_event_data_by_contract(265598)
        .fetch()
        .await
        .expect("wsh_event_data_by_contract failed");
    assert!(!data.data_json.is_empty());
}

#[tokio::test]
#[ignore]
async fn wsh_event_data_by_filter() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let mut subscription = client
        .wsh_event_data_by_filter("{}")
        .subscribe()
        .await
        .expect("wsh_event_data_by_filter failed");
    let _item = subscription.next().await;
}
