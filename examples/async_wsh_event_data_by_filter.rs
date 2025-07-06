//! # WSH Event Data by Filter Example (Async)
//! 
//! This example demonstrates how to stream Wall Street Horizon event data
//! using filters with the async API. This allows you to receive continuous
//! updates for events matching specific criteria.
//!
//! To run this example:
//! ```bash
//! cargo run --features async --example async_wsh_event_data_by_filter
//! ```
//!
//! Make sure TWS or IB Gateway is running with API connections enabled

use ibapi::prelude::*;
use ibapi::wsh::AutoFill;

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

    // Define filter for events
    // This is typically a JSON-encoded filter string
    // Example: Get earnings events for US companies
    let filter = r#"{
        "country": "US",
        "eventType": "Earnings",
        "watchlist": true
    }"#;
    
    // Limit number of events
    let limit = Some(50);
    
    // Configure autofill options
    let auto_fill = Some(AutoFill {
        competitors: false,
        portfolio: true,    // Include portfolio positions
        watchlist: true,    // Include watchlist items
    });

    // Request WSH event data stream
    match client.wsh_event_data_by_filter(filter, limit, auto_fill).await {
        Ok(mut event_stream) => {
            println!("\nStreaming WSH events with filter:");
            println!("Filter: {}", filter);
            println!("Waiting for events...\n");
            
            let mut event_count = 0;
            
            // Process events as they arrive
            while let Some(event_result) = event_stream.next().await {
                match event_result {
                    Ok(event_data) => {
                        event_count += 1;
                        println!("\nEvent #{} received:", event_count);
                        println!("{}", event_data.data_json);
                        
                        // Limit output for demo purposes
                        if event_count >= 10 {
                            println!("\nReceived {} events. Stopping demo.", event_count);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving event: {}", e);
                        break;
                    }
                }
            }
            
            if event_count == 0 {
                println!("No events received. Check your filter criteria.");
            }
        }
        Err(e) => {
            eprintln!("Error requesting WSH event stream: {}", e);
        }
    }
}