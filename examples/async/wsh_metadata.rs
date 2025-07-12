//! # WSH Metadata Example (Async)
//!
//! This example demonstrates how to retrieve Wall Street Horizon metadata
//! using the async API. WSH metadata contains configuration and setup
//! information for the Wall Street Horizon data service.
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_wsh_metadata
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    // Connect to TWS or IB Gateway
    let client = match Client::connect("127.0.0.1:4002", 100).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect: {e:?}");
            return;
        }
    };

    println!("Connected to TWS/Gateway");
    println!("Server Version: {}", client.server_version());

    // Request WSH metadata
    match client.wsh_metadata().await {
        Ok(metadata) => {
            println!("\nWSH Metadata received:");
            println!("{}", metadata.data_json);
        }
        Err(e) => {
            eprintln!("Error requesting WSH metadata: {e:?}");
        }
    }
}
