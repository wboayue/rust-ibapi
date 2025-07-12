//! Sync Account Summary example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example account_summary
//! ```

use ibapi::accounts::{types::AccountGroup, AccountSummaries, AccountSummaryTags};
use ibapi::Client;

fn main() {
    env_logger::init();

    println!("Connecting to IB Gateway...");

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    println!("Connected successfully!");

    println!("\nRequesting account summary...");
    let tags = &[
        AccountSummaryTags::ACCOUNT_TYPE,
        AccountSummaryTags::NET_LIQUIDATION,
        AccountSummaryTags::TOTAL_CASH_VALUE,
        AccountSummaryTags::BUYING_POWER,
    ];

    let subscription = client
        .account_summary(&AccountGroup("All".to_string()), tags)
        .expect("error requesting account summary");

    for update in &subscription {
        match update {
            AccountSummaries::Summary(summary) => {
                if summary.currency.is_empty() {
                    println!("Account {}: {} = {}", summary.account, summary.tag, summary.value);
                } else {
                    println!("Account {}: {} = {} {}", summary.account, summary.tag, summary.value, summary.currency);
                }
            }
            AccountSummaries::End => {
                println!("Account summary complete.");
                subscription.cancel();
            }
        }
    }
}
