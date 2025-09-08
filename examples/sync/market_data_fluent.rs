//! Market Data Fluent API example
//!
//! This example demonstrates the new fluent API for subscribing to market data.
//! The fluent interface provides better discoverability and cleaner code compared
//! to the raw API.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_data_fluent
//! ```

use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL").build();

    println!("Subscribing to market data for AAPL using the fluent API...\n");

    // Example 1: Subscribe to streaming market data with specific tick types
    println!("Example 1: Streaming with specific tick types");
    let subscription = client
        .market_data(&contract)
        .generic_ticks(&["233", "236", "293"]) // RTVolume, Shortable, Trade Count
        .subscribe()
        .expect("Failed to subscribe to market data");

    let mut tick_count = 0;
    for tick in &subscription {
        match tick {
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
        }

        tick_count += 1;
        if tick_count >= 10 {
            println!("\nReceived 10 ticks, cancelling subscription...");
            subscription.cancel();
            break;
        }
    }

    println!("\n" + "=".repeat(50).as_str() + "\n");

    // Example 2: Request a one-time snapshot
    println!("Example 2: One-time snapshot");
    let snapshot_subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .expect("Failed to request snapshot");

    for tick in &snapshot_subscription {
        match tick {
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
        }
    }

    println!("\n" + "=".repeat(50).as_str() + "\n");

    // Example 3: Switching from snapshot to streaming
    println!("Example 3: Combining builder methods");
    let combined_subscription = client
        .market_data(&contract)
        .generic_ticks(&["100", "101", "104"]) // Option Volume, Open Interest, Historical Vol
        .snapshot()      // Initially set to snapshot
        .streaming()     // Override to streaming mode
        .subscribe()
        .expect("Failed to subscribe");

    println!("This subscription is in streaming mode (not snapshot)");
    
    let mut tick_count = 0;
    for tick in &combined_subscription {
        if let TickTypes::Generic(generic) = tick {
            println!("Generic tick - Type: {:?}, Value: {:.2}", generic.tick_type, generic.value);
            tick_count += 1;
            if tick_count >= 5 {
                combined_subscription.cancel();
                break;
            }
        }
    }

    println!("\nAll examples completed!");
}