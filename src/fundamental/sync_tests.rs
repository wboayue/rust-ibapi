use std::sync::{Arc, RwLock};

use crate::client::blocking::Client;
use crate::common::test_utils::helpers::{assert_request, proto_response, TEST_CONTRACT_ID, TEST_REQ_ID_FIRST};
use crate::contracts::Contract;
use crate::fundamental::FundamentalReportType;
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::fundamental::{fundamental_data_request, fundamental_data_response};
use crate::testdata::builders::ResponseProtoEncoder;
use crate::Error;

fn aapl_contract() -> Contract {
    let mut contract = Contract::stock("AAPL").build();
    contract.contract_id = TEST_CONTRACT_ID;
    contract
}

#[test]
fn fundamental_data_round_trip() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![proto_response(
            IncomingMessages::FundamentalData,
            fundamental_data_response()
                .request_id(TEST_REQ_ID_FIRST)
                .data("<ReportSnapshot>...</ReportSnapshot>")
                .encode_proto(),
        )],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::FUNDAMENTAL_DATA);
    let contract = aapl_contract();

    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
        .expect("fundamental_data failed");

    assert_request(
        &message_bus,
        0,
        &fundamental_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(contract.clone())
            .report_type(FundamentalReportType::ReportSnapshot),
    );

    assert_eq!(report.data, "<ReportSnapshot>...</ReportSnapshot>");
}

#[test]
fn fundamental_data_propagates_tws_error() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![crate::common::test_utils::helpers::proto_error_response(
            TEST_REQ_ID_FIRST,
            10089,
            "Requested market data is not subscribed",
        )],
    });

    let client = Client::stubbed(message_bus, server_versions::FUNDAMENTAL_DATA);
    let contract = aapl_contract();
    let err = client
        .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
        .expect_err("expected TWS error");
    match err {
        Error::Notice(n) => {
            assert_eq!(n.code, 10089);
            assert!(n.message.contains("not subscribed"));
        }
        other => panic!("expected Error::Notice, got {other:?}"),
    }
}
