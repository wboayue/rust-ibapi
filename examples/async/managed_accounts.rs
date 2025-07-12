#![allow(clippy::uninlined_format_args)]
//! Example of getting managed accounts asynchronously
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_managed_accounts
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting to IB Gateway...");

    // Connect to Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected successfully!");

    // Request managed accounts
    println!("\nRequesting managed accounts...");
    let accounts = client.managed_accounts().await?;

    if accounts.is_empty() {
        println!("No managed accounts found.");
    } else {
        println!("Managed accounts:");
        for account in accounts {
            println!("  - {account}");
        }
    }

    Ok(())
}
