//! Scanner Subscription Complex Orders example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example scanner_subscription_complex_orders
//! ```

use ibapi::client::blocking::Client;
use ibapi::{contracts, scanner};

// This example demonstrates setting up a market scanner.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let scanner_subscription = complex_orders_and_trades();

    let subscription = client
        .scanner_subscription(&scanner_subscription)
        .expect("request scanner parameters failed");

    match subscription.next_data() {
        Some(Ok(scan_results)) => {
            for scan_data in scan_results.iter() {
                println!(
                    "rank: {}, contract_id: {}, symbol: {}",
                    scan_data.rank, scan_data.contract_details.contract.contract_id, scan_data.contract_details.contract.symbol
                );
            }
        }
        Some(Err(e)) => eprintln!("scanner error: {e}"),
        None => eprintln!("scanner subscription ended without results"),
    }
}

fn complex_orders_and_trades() -> scanner::ScannerSubscription {
    scanner::ScannerSubscription {
        location_code: Some("NATCOMB.OPT.US".to_string()),
        filter_options: vec![contracts::TagValue {
            tag: "underConID".to_string(),
            value: "265598".to_string(),
        }],
        ..scanner::ScannerSubscription::new("NATCOMB", "COMBO_LATEST_TRADE")
    }
}
