use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::fundamental::FundamentalReportType;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
#[ignore] // requires fundamental-data entitlement on the connected paper-trading account
fn fundamental_data_snapshot() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
        .expect("fundamental_data failed");
    assert!(!report.data.is_empty(), "report.data should be non-empty XML");
}

#[test]
#[ignore]
fn fundamental_data_ratios() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportRatios)
        .expect("fundamental_data failed");
    assert!(!report.data.is_empty());
}
