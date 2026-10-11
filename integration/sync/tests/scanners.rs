use ibapi::scanner::ScannerSubscription;
use ibapi_integration_sync::connect;
use ibapi_test::ClientId;

#[test]
fn scanner_parameters_returns_xml() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let xml = client.scanner_parameters().expect("scanner_parameters failed");

    assert!(!xml.is_empty());
    assert!(xml.starts_with("<?xml"), "expected XML content");
}

#[test]
fn scanner_subscription_top_gainers() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let sub = ScannerSubscription {
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US.MAJOR".to_string()),
        number_of_rows: 10,
        ..Default::default()
    };

    let subscription = client.scanner_subscription(&sub, &[]).expect("scanner_subscription failed");

    let results = subscription
        .next_data()
        .expect("expected scanner results")
        .expect("scanner subscription error");
    assert!(!results.is_empty(), "expected non-empty scanner results");
}
