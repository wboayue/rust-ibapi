//! Scanner Subscription Complex Orders example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example scanner_subscription_complex_orders
//! ```

use ibapi::{orders, scanner, Client};

// This example demonstrates setting up a market scanner.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let scanner_subscription = complex_orders_and_trades();
    let filter = vec![orders::TagValue {
        tag: "underConID".to_string(),
        value: "265598".to_string(),
    }];

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

fn complex_orders_and_trades() -> scanner::ScannerSubscription {
    scanner::ScannerSubscription {
        instrument: Some("NATCOMB".to_string()),
        location_code: Some("NATCOMB.OPT.US".to_string()),
        scan_code: Some("COMBO_LATEST_TRADE".to_string()),
        ..Default::default()
    }
}
