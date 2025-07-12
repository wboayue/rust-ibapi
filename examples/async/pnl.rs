//! Example of getting real-time PnL updates asynchronously
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_pnl
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Get account from command line or use default
    let account = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <account>", env::args().next().unwrap());
        eprintln!("Using default account 'DU1234567'");
        "DU1234567".to_string()
    });

    println!("Connecting to IB Gateway...");

    // Connect to Gateway
    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected successfully!");

    // Request PnL updates
    println!("\nRequesting PnL updates for account {account}...");
    let mut subscription = client.pnl(&account, None).await?;

    // Process PnL updates
    println!("Waiting for PnL updates (press Ctrl+C to stop)...");
    while let Some(result) = subscription.next().await {
        match result {
            Ok(pnl_update) => {
                print!("PnL Update - Daily: ${:.2}", pnl_update.daily_pnl);

                if let Some(unrealized) = pnl_update.unrealized_pnl {
                    print!(", Unrealized: ${unrealized:.2}");
                }

                if let Some(realized) = pnl_update.realized_pnl {
                    print!(", Realized: ${realized:.2}");
                }

                println!();
            }
            Err(e) => {
                eprintln!("Error receiving PnL: {e}");
                break;
            }
        }
    }

    Ok(())
}
