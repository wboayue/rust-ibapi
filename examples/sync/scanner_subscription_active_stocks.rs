//! Scanner Subscription Active Stocks example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example scanner_subscription_active_stocks
//! ```

use ibapi::{scanner, Client};

// This example demonstrates setting up a market scanner.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let scanner_subscription = most_active_stocks(10);
    let filter = Vec::new();

    let subscription = client
        .scanner_subscription(&scanner_subscription, &filter)
        .expect("request scanner parameters failed");

    if let Some(scan_results) = subscription.next() {
        for scan_data in scan_results.iter() {
            println!(
                "rank: {}, contract_id: {}, symbol: {}",
                scan_data.rank, scan_data.contract_details.contract.contract_id, scan_data.contract_details.contract.symbol
            );
        }
    }
}

fn most_active_stocks(number_of_rows: i32) -> scanner::ScannerSubscription {
    scanner::ScannerSubscription {
        number_of_rows,
        instrument: Some("STK".to_string()),
        location_code: Some("STK.US.MAJOR".to_string()),
        scan_code: Some("MOST_ACTIVE".to_string()),
        ..Default::default()
    }
}
