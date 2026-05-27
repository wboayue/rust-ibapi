use ibapi::contracts::Contract;
use ibapi::fundamental::FundamentalReportType;
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
#[ignore] // requires fundamental-data entitlement on the connected paper-trading account
async fn fundamental_data_snapshot() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
        .await
        .expect("fundamental_data failed");
    assert!(!report.data.is_empty(), "report.data should be non-empty XML");
}

#[tokio::test]
#[ignore]
async fn fundamental_data_financial_summary() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    rate_limit();
    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportsFinSummary)
        .await
        .expect("fundamental_data failed");
    assert!(!report.data.is_empty());
}
