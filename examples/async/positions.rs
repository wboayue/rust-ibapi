#![allow(clippy::uninlined_format_args)]
//! Example of getting account positions asynchronously
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_positions
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

    // Request positions for all accounts
    println!("\nRequesting positions...");
    let mut subscription = client.positions().await?;

    // Process position updates
    while let Some(result) = subscription.next().await {
        match result {
            Ok(update) => match update {
                PositionUpdate::Position(position) => {
                    println!(
                        "Position: {} {} @ {} (avg cost: {}) in account {}",
                        position.position, position.contract.symbol, position.contract.exchange, position.average_cost, position.account
                    );
                }
                PositionUpdate::PositionEnd => {
                    println!("All positions received.");
                    break;
                }
            },
            Err(e) => {
                eprintln!("Error receiving position: {e:?}");
                break;
            }
        }
    }

    Ok(())
}
