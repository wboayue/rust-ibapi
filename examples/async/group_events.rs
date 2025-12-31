#![allow(clippy::uninlined_format_args)]
//! Example of subscribing to TWS display group events asynchronously
//!
//! Display Groups are a TWS-only feature (not available in IB Gateway).
//! They allow organizing contracts into color-coded groups in the TWS UI.
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_group_events
//! ```
//!
//! Make sure TWS is running with API connections enabled

use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting to TWS...");

    // Display Groups require TWS (not IB Gateway)
    let client = Client::connect("127.0.0.1:7497", 100).await?;
    println!("Connected successfully!");

    println!("\nSubscribing to group events for group 1...");
    // 1 corresponds to "Group 1" in TWS (Red)
    let mut subscription = client.subscribe_to_group_events(1).await?;

    println!("Listening for events. Change the contract in TWS Group 1 (Red) to see updates.");

    while let Some(result) = subscription.next().await {
        match result {
            Ok(contract_info) => {
                println!("Received group event: {}", contract_info);
            }
            Err(e) => {
                eprintln!("Error receiving group event: {e:?}");
                break;
            }
        }
    }

    Ok(())
}
