use ibapi::orders::TagValue;
use ibapi::scanner::ScannerSubscription;
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
async fn scanner_parameters_returns_xml() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let xml = client.scanner_parameters().await.expect("scanner_parameters failed");

    assert!(!xml.is_empty());
    assert!(xml.starts_with("<?xml"), "expected XML content");
}

#[tokio::test]
async fn scanner_subscription_top_gainers() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    let mut sub = ScannerSubscription::default();
    sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    sub.instrument = Some("STK".to_string());
    sub.location_code = Some("STK.US.MAJOR".to_string());
    sub.number_of_rows = 10;

    rate_limit();
    let mut subscription = client.scanner_subscription(&sub, &vec![]).await.expect("scanner_subscription failed");

    let item = subscription.next().await;
    assert!(item.is_some(), "expected scanner results");
    let results = item.unwrap().expect("scanner error");
    assert!(!results.is_empty(), "expected non-empty scanner results");
}
