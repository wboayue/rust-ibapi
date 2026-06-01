#![allow(clippy::uninlined_format_args)]
//! # Fundamental Data Example (Async)
//!
//! Requests a Reuters fundamental-data report (snapshot, financials,
//! ratios, etc.) for a contract. The response is an XML payload.
//!
//! ```bash
//! cargo run --example async_fundamental_data
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled.

use ibapi::contracts::Contract;
use ibapi::fundamental::FundamentalReportType;
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = match Client::connect("127.0.0.1:4002", 100).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect: {e:?}");
            return;
        }
    };

    println!("Connected to TWS/Gateway");
    println!("Server Version: {}", client.server_version());

    let contract = Contract::stock("AAPL").build();
    match client.fundamental_data(&contract, FundamentalReportType::ReportSnapshot).await {
        Ok(report) => {
            println!("\nFundamental data received:");
            println!("{}", report.data);
        }
        Err(e) => {
            eprintln!("Error requesting fundamental data: {e:?}");
        }
    }
}
