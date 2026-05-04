use super::*;
use crate::common::test_utils::helpers::{assert_request, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::{Exchange, SecurityType, Symbol};
use crate::orders::TagValue;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::scanner::{cancel_scanner_subscription_request, scanner_parameters_request, scanner_subscription_request};
use std::sync::{Arc, RwLock};

#[tokio::test]
async fn test_scanner_parameters() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "19|2|<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ScanParameterResponse>\n<InstrumentList>...</InstrumentList>\n</ScanParameterResponse>".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SCANNER_GENERIC_OPTS);

    let scanner_params = client.scanner_parameters().await.expect("request scanner parameters failed");

    assert_request(&message_bus, 0, &scanner_parameters_request());

    assert!(scanner_params.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(scanner_params.contains("<ScanParameterResponse>"));
    assert!(scanner_params.contains("<InstrumentList>"));
}

#[tokio::test]
async fn test_scanner_subscription() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "20\03\09000\010\00\0670777621\0SVMH\0STK\0\00\0\0SMART\0USD\0SVMH\0NMS\0NMS\0\0\0\0\01\0536918651\0GTI\0STK\0\00\0\0SMART\0USD\0GTI\0NMS\0NMS\0\0\0\0\02\0526726639\0LITM\0STK\0\00\0\0SMART\0USD\0LITM\0SCM\0SCM\0\0\0\0\03\0504716446\0LCID\0STK\0\00\0\0SMART\0USD\0LCID\0NMS\0NMS\0\0\0\0\04\0547605251\0RGTI\0STK\0\00\0\0SMART\0USD\0RGTI\0SCM\0SCM\0\0\0\0\05\0653568762\0AVGR\0STK\0\00\0\0SMART\0USD\0AVGR\0SCM\0SCM\0\0\0\0\06\04815747\0NVDA\0STK\0\00\0\0SMART\0USD\0NVDA\0NMS\0NMS\0\0\0\0\07\0534453483\0HOUR\0STK\0\00\0\0SMART\0USD\0HOUR\0SCM\0SCM\0\0\0\0\08\0631370187\0LAES\0STK\0\00\0\0SMART\0USD\0LAES\0SCM\0SCM\0\0\0\0\09\0689954925\0XTIA\0STK\0\00\0\0SMART\0USD\0XTIA\0SCM\0SCM\0\0\0\0\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SCANNER_GENERIC_OPTS);

    let subscription_params = ScannerSubscription {
        number_of_rows: 10,
        instrument: Some("FUT".to_string()),
        location_code: Some("FUT.US".to_string()),
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        above_price: Some(50.0),
        below_price: Some(100.0),
        above_volume: Some(1000),
        average_option_volume_above: Some(100),
        market_cap_above: Some(1000000.0),
        market_cap_below: Some(10000000.0),
        moody_rating_above: Some("A".to_string()),
        moody_rating_below: Some("AAA".to_string()),
        sp_rating_above: Some("A".to_string()),
        sp_rating_below: Some("AAA".to_string()),
        maturity_date_above: Some("20230101".to_string()),
        maturity_date_below: Some("20231231".to_string()),
        coupon_rate_above: Some(2.0),
        coupon_rate_below: Some(5.0),
        exclude_convertible: true,
        scanner_setting_pairs: Some("Annual,true".to_string()),
        stock_type_filter: Some("CORP".to_string()),
    };

    let filter = vec![
        TagValue {
            tag: "scannerType".to_string(),
            value: "TOP_PERC_GAIN".to_string(),
        },
        TagValue {
            tag: "numberOfRows".to_string(),
            value: "10".to_string(),
        },
    ];

    let mut subscription = client
        .scanner_subscription(&subscription_params, &filter)
        .await
        .expect("request scanner subscription failed");

    let scanner_data = match subscription.next_data().await {
        Some(Ok(data)) => data,
        Some(Err(e)) => panic!("Error getting scanner results: {e}"),
        None => panic!("Unexpected end of stream"),
    };

    assert_eq!(scanner_data.len(), 10);

    let first = &scanner_data[0];
    assert_eq!(first.rank, 0);
    assert_eq!(first.contract_details.contract.symbol, Symbol::from("SVMH"));
    assert_eq!(first.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(first.contract_details.contract.exchange, Exchange::from("SMART"));

    let second = &scanner_data[1];
    assert_eq!(second.rank, 1);
    assert_eq!(second.contract_details.contract.symbol, Symbol::from("GTI"));
    assert_eq!(second.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(second.contract_details.contract.exchange, Exchange::from("SMART"));

    let third = &scanner_data[2];
    assert_eq!(third.rank, 2);
    assert_eq!(third.contract_details.contract.symbol, Symbol::from("LITM"));
    assert_eq!(third.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(third.contract_details.contract.exchange, Exchange::from("SMART"));

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &scanner_subscription_request()
            .request_id(TEST_REQ_ID_FIRST)
            .subscription(&subscription_params)
            .filter(&filter),
    );

    subscription.cancel().await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 1, &cancel_scanner_subscription_request().request_id(TEST_REQ_ID_FIRST));
}

#[tokio::test]
async fn test_scanner_subscription_drop_sends_cancel() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["20\03\09000\01\00\0670777621\0SVMH\0STK\0\00\0\0SMART\0USD\0SVMH\0NMS\0NMS\0\0\0\0\0".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SCANNER_GENERIC_OPTS);

    let subscription_params = ScannerSubscription {
        number_of_rows: 1,
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        ..Default::default()
    };

    let mut subscription = client
        .scanner_subscription(&subscription_params, &[])
        .await
        .expect("request scanner subscription failed");

    let _ = subscription.next_data().await;

    assert_eq!(request_message_count(&message_bus), 1);

    drop(subscription);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(&message_bus, 1, &cancel_scanner_subscription_request().request_id(TEST_REQ_ID_FIRST));
}

#[tokio::test]
async fn test_scanner_subscription_no_double_cancel() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["20\03\09000\01\00\0670777621\0SVMH\0STK\0\00\0\0SMART\0USD\0SVMH\0NMS\0NMS\0\0\0\0\0".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SCANNER_GENERIC_OPTS);

    let subscription_params = ScannerSubscription {
        number_of_rows: 1,
        scan_code: Some("TOP_PERC_GAIN".to_string()),
        ..Default::default()
    };

    let subscription = client
        .scanner_subscription(&subscription_params, &[])
        .await
        .expect("request scanner subscription failed");

    subscription.cancel().await;

    assert_eq!(request_message_count(&message_bus), 2);

    drop(subscription);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert_eq!(request_message_count(&message_bus), 2, "no double cancel");
}
