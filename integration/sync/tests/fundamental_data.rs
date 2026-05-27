use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::fundamental::FundamentalReportType;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[test]
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
fn fundamental_data_financial_summary() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportsFinSummary)
        .expect("fundamental_data failed");
    assert!(!report.data.is_empty());
}
