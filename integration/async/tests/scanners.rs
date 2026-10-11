use futures::StreamExt;
use ibapi::scanner::ScannerSubscription;
use ibapi::subscriptions::SubscriptionItemStreamExt;
use ibapi_integration_async::connect;
use ibapi_test::ClientId;

#[tokio::test]
async fn scanner_parameters_returns_xml() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let xml = tokio::time::timeout(tokio::time::Duration::from_secs(30), client.scanner_parameters())
        .await
        .expect("scanner_parameters timed out")
        .expect("scanner_parameters failed");

    assert!(!xml.is_empty());
    assert!(xml.starts_with("<?xml"), "expected XML content");
}

#[tokio::test]
async fn scanner_subscription_top_gainers() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let sub = ScannerSubscription {
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US.MAJOR".to_string()),
        number_of_rows: 10,
        ..Default::default()
    };

    let mut subscription = client.scanner_subscription(&sub, &[]).await.expect("scanner_subscription failed");

    let results = (&mut subscription)
        .filter_data()
        .next()
        .await
        .expect("expected scanner results")
        .expect("scanner subscription error");
    assert!(!results.is_empty(), "expected non-empty scanner results");
}
