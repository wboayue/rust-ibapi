//! Market Data Fluent API example (async)
//!
//! This example demonstrates the new fluent API for subscribing to market data
//! using the async client. The fluent interface provides better discoverability
//! and cleaner code compared to the raw API.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features async --example market_data_fluent
//! ```

use ibapi::prelude::*;
use ibapi::client::r#async::Client;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)
        .await
        .expect("connection failed");

    let contract = Contract::stock("AAPL").build();

    println!("Subscribing to market data for AAPL using the fluent API (async)...\n");

    // Example 1: Subscribe to streaming market data with specific tick types
    println!("Example 1: Streaming with specific tick types");
    let mut subscription = client
        .market_data(&contract)
        .generic_ticks(&["233", "236", "293"]) // RTVolume, Shortable, Trade Count
        .subscribe()
        .await
        .expect("Failed to subscribe to market data");

    let mut tick_count = 0;
    while let Some(result) = subscription.next().await {
        match result {
            Ok(tick) => match tick {
                TickTypes::Price(price) => {
                    println!("Price - Type: {:?}, Value: ${:.2}", price.tick_type, price.price);
                }
                TickTypes::Size(size) => {
                    println!("Size - Type: {:?}, Value: {:.0}", size.tick_type, size.size);
                }
                TickTypes::String(string) => {
                    println!("String - Type: {:?}, Value: {}", string.tick_type, string.value);
                }
                TickTypes::Generic(generic) => {
                    println!("Generic - Type: {:?}, Value: {:.2}", generic.tick_type, generic.value);
                }
                TickTypes::Notice(notice) => {
                    println!("Notice - Code: {}, Message: {}", notice.code, notice.message);
                }
                _ => {}
            },
            Err(e) => {
                eprintln!("Error receiving tick: {:?}", e);
            }
        }

        tick_count += 1;
        if tick_count >= 10 {
            println!("\nReceived 10 ticks, cancelling subscription...");
            subscription.cancel().await;
            break;
        }
    }

    println!("\n" + "=".repeat(50).as_str() + "\n");

    // Example 2: Request a one-time snapshot
    println!("Example 2: One-time snapshot");
    let mut snapshot_subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .await
        .expect("Failed to request snapshot");

    while let Some(result) = snapshot_subscription.next().await {
        match result {
            Ok(tick) => match tick {
                TickTypes::Price(price) => {
                    println!("Snapshot Price - Type: {:?}, Value: ${:.2}", price.tick_type, price.price);
                }
                TickTypes::Size(size) => {
                    println!("Snapshot Size - Type: {:?}, Value: {:.0}", size.tick_type, size.size);
                }
                TickTypes::SnapshotEnd => {
                    println!("Snapshot completed!");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                eprintln!("Error receiving tick: {:?}", e);
            }
        }
    }

    println!("\n" + "=".repeat(50).as_str() + "\n");

    // Example 3: Using the build() alias
    println!("Example 3: Using build() alias method");
    let mut build_subscription = client
        .market_data(&contract)
        .generic_ticks(&["100", "101"]) // Option Volume, Open Interest
        .build()  // build() is an alias for subscribe()
        .await
        .expect("Failed to subscribe");

    println!("Subscribed using build() method (alias for subscribe())");
    
    let mut tick_count = 0;
    while let Some(result) = build_subscription.next().await {
        if let Ok(TickTypes::Generic(generic)) = result {
            println!("Generic tick - Type: {:?}, Value: {:.2}", generic.tick_type, generic.value);
            tick_count += 1;
            if tick_count >= 5 {
                build_subscription.cancel().await;
                break;
            }
        }
    }

    println!("\nAll examples completed!");
}