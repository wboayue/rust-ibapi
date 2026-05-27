#![allow(clippy::uninlined_format_args)]
//! # Smart Components Example (Async)
//!
//! Requests the underlying exchanges that contribute to a consolidated (BBO)
//! feed. Pass a BBO exchange code as the first argument (defaults to
//! `"a6"`). The BBO code is an opaque per-session token typically obtained
//! from the `LAST_EXCHANGE` market-data tick (tick type 84); `"a6"` is the
//! canonical sample value used in the IBKR Python testbed.
//!
//! ```bash
//! cargo run --example async_smart_components -- a6
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled.

use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let bbo_exchange = std::env::args().nth(1).unwrap_or_else(|| "a6".to_string());

    let client = match Client::connect("127.0.0.1:4002", 100).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect: {e:?}");
            return;
        }
    };

    println!("Connected to TWS/Gateway");
    println!("Server Version: {}", client.server_version());

    match client.smart_components(&bbo_exchange).await {
        Ok(components) => {
            println!("\nSmart components for {bbo_exchange}:");
            for component in &components {
                println!("  bit {}: {} ({})", component.bit_number, component.exchange, component.exchange_letter);
            }
        }
        Err(e) => {
            eprintln!("Error requesting smart components: {e:?}");
        }
    }
}
