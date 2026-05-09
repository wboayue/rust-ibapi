use super::*;
use crate::client::blocking::Client;
use crate::common::test_utils::helpers::{assert_request, proto_response, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::{Exchange, SecurityType, Symbol};
use crate::messages::IncomingMessages;
use crate::orders::TagValue;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::scanner::{
    cancel_scanner_subscription_request, scanner_data, scanner_data_row, scanner_parameters, scanner_parameters_request, scanner_subscription_request,
};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;

#[test]
fn test_scanner_parameters() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::ScannerParameters,
        scanner_parameters().encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_SCAN_DATA);

    let scanner_params = client.scanner_parameters().expect("request scanner parameters failed");

    assert_request(&message_bus, 0, &scanner_parameters_request());

    assert!(scanner_params.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(scanner_params.contains("<ScanParameterResponse>"));
    assert!(scanner_params.contains("<InstrumentList>"));
}

#[test]
fn test_scanner_subscription() {
    let rows = vec![
        scanner_data_row(0, 670777621, "SVMH"),
        scanner_data_row(1, 536918651, "GTI"),
        scanner_data_row(2, 526726639, "LITM").market_name("SCM"),
    ];

    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::ScannerData,
        scanner_data().request_id(TEST_REQ_ID_FIRST).rows(rows).encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::PROTOBUF_SCAN_DATA);

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

    let subscription = client
        .scanner_subscription(&subscription_params, &filter)
        .expect("request scanner subscription failed");

    let scanner_data: Vec<Vec<ScannerData>> = subscription.iter_data().map(|r| r.expect("error getting scanner results")).collect();

    assert_eq!(scanner_data.len(), 1);

    let first = &scanner_data[0][0];
    assert_eq!(first.rank, 0);
    assert_eq!(first.contract_details.contract.symbol, Symbol::from("SVMH"));
    assert_eq!(first.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(first.contract_details.contract.exchange, Exchange::from("SMART"));

    let second = &scanner_data[0][1];
    assert_eq!(second.rank, 1);
    assert_eq!(second.contract_details.contract.symbol, Symbol::from("GTI"));
    assert_eq!(second.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(second.contract_details.contract.exchange, Exchange::from("SMART"));

    let third = &scanner_data[0][2];
    assert_eq!(third.rank, 2);
    assert_eq!(third.contract_details.contract.symbol, Symbol::from("LITM"));
    assert_eq!(third.contract_details.contract.security_type, SecurityType::Stock);
    assert_eq!(third.contract_details.contract.exchange, Exchange::from("SMART"));

    drop(subscription);

    assert_eq!(request_message_count(&message_bus), 2);
    assert_request(
        &message_bus,
        0,
        &scanner_subscription_request()
            .request_id(TEST_REQ_ID_FIRST)
            .subscription(&subscription_params)
            .filter(&filter),
    );
    assert_request(&message_bus, 1, &cancel_scanner_subscription_request().request_id(TEST_REQ_ID_FIRST));
}
