#![allow(clippy::uninlined_format_args)]
//! Async historical schedules example
//!
//! This example demonstrates how to retrieve trading schedule information
//! for a contract using the async API. It exercises the
//! `historical_schedules` builder both without `.ending()` (anchors at
//! current time) and with `.ending(date)` (anchors at a specific end date).
//!
//! # Usage
//!
//! Make sure IB Gateway or TWS is running with API connections enabled, then run:
//!
//! ```bash
//! cargo run --features async --example async_historical_schedules
//! ```
//!
//! # Configuration
//!
//! - Adjust the connection address if needed (default: 127.0.0.1:4002)
//! - Change the contract and time period as desired

use std::sync::Arc;

use ibapi::{
    contracts::{Contract, ContractMonth},
    market_data::historical::ToDuration,
    Client,
};
use time::macros::datetime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway (port 4002) or TWS (port 7497)
    let client = Arc::new(Client::connect("127.0.0.1:4002", 102).await?);
    println!("Connected to IB Gateway");

    // Create different contract types
    let contracts = vec![
        ("AAPL", Contract::stock("AAPL").build(), "NASDAQ"),
        ("SPY", Contract::stock("SPY").build(), "NYSE"),
        (
            "GC",
            Contract::futures("GC")
                .expires_in(ContractMonth::new(2025, 2))
                .on_exchange("COMEX")
                .build(),
            "COMEX",
        ),
    ];

    // Request trading schedule for each contract
    for (name, contract, exchange) in contracts {
        println!("\n{name} Trading Schedule ({exchange}):");

        // Get last 30 days of trading schedule, anchored to current time
        let duration = 30.days();

        match client.historical_schedules(&contract, duration).fetch().await {
            Ok(schedule) => {
                println!("  Schedule from {} to {}", schedule.start, schedule.end);
                println!("  Timezone: {}", schedule.time_zone);

                // Show last 5 sessions
                let session_count = schedule.sessions.len();
                let sessions_to_show = session_count.min(5);

                println!("\n  Last {sessions_to_show} trading sessions:");
                for session in schedule.sessions.iter().rev().take(sessions_to_show).rev() {
                    println!("    {} - Trading: {} to {}", session.reference, session.start, session.end);
                }

                println!("  Total sessions in period: {session_count}");
            }
            Err(e) => {
                println!("  Error: {e:?}");
            }
        }

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Example with specific date range
    println!("\n\nSpecific date range example (Thanksgiving week 2023):");
    let contract = Contract::stock("AAPL").build();
    let end_date = datetime!(2023-11-26 00:00 UTC);
    let duration = 7.days();

    match client.historical_schedules(&contract, duration).ending(end_date).fetch().await {
        Ok(schedule) => {
            println!("Schedule for Thanksgiving week:");
            for session in &schedule.sessions {
                println!("  {} - Trading: {} to {}", session.reference, session.start, session.end);
            }
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }

    println!("\nExample completed!");
    Ok(())
}
