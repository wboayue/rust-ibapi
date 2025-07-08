//! # WSH Event Data by Contract Example (Async)
//!
//! This example demonstrates how to retrieve Wall Street Horizon event data
//! for a specific contract using the async API. This includes earnings
//! calendars, corporate events, and other fundamental data events.
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_wsh_event_data_by_contract
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::prelude::*;
use ibapi::wsh::AutoFill;
use time::macros::date;

#[tokio::main]
async fn main() {
    env_logger::init();

    // Connect to TWS or IB Gateway
    let client = match Client::connect("127.0.0.1:4002", 100).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    };

    println!("Connected to TWS/Gateway");
    println!("Server Version: {}", client.server_version());

    // Example: Get WSH events for AAPL (contract ID would be obtained from contract details)
    let contract_id = 265598; // AAPL contract ID (example - verify with contract details)

    // Set date range for events
    let start_date = Some(date!(2024 - 01 - 01));
    let end_date = Some(date!(2024 - 12 - 31));
    let limit = Some(100);

    // Configure autofill options
    let auto_fill = Some(AutoFill {
        competitors: true, // Include competitor events
        portfolio: false,  // Don't include portfolio positions
        watchlist: false,  // Don't include watchlist items
    });

    // Request WSH event data
    match client
        .wsh_event_data_by_contract(contract_id, start_date, end_date, limit, auto_fill)
        .await
    {
        Ok(event_data) => {
            println!("\nWSH Event Data received:");
            println!("{}", event_data.data_json);
        }
        Err(e) => {
            eprintln!("Error requesting WSH event data: {}", e);
        }
    }
}
