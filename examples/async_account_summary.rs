//! Example of getting account summary asynchronously
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_account_summary
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::accounts::{account_summary, AccountSummaries, AccountSummaryTags};
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting to IB Gateway...");

    // Connect to Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected successfully!");

    // Request account summary for all accounts
    println!("\nRequesting account summary...");
    let tags = &[
        AccountSummaryTags::ACCOUNT_TYPE,
        AccountSummaryTags::NET_LIQUIDATION,
        AccountSummaryTags::TOTAL_CASH_VALUE,
        AccountSummaryTags::BUYING_POWER,
    ];

    let mut subscription = account_summary(&client, "All", tags).await?;

    // Process account summary updates
    while let Some(result) = subscription.next().await {
        match result {
            Ok(update) => match update {
                AccountSummaries::Summary(summary) => {
                    println!("Account {}: {} = {} {}", summary.account, summary.tag, summary.value, summary.currency);
                }
                AccountSummaries::End => {
                    println!("Account summary complete.");
                    break;
                }
            },
            Err(e) => {
                eprintln!("Error receiving account summary: {}", e);
                break;
            }
        }
    }

    Ok(())
}
