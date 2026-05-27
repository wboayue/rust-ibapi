//! Fundamental data example (sync).
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example fundamental_data
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::fundamental::FundamentalReportType;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL").build();
    let report = client
        .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
        .expect("request fundamental data failed");
    println!("{}", report.data);
}
