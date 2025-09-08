#![allow(clippy::uninlined_format_args)]
//! Async Market Data example
//!
//! This example demonstrates how to subscribe to market data using the async API.
//! It shows streaming data, snapshots, and builder method chaining.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features async --example async_market_data
//! ```

use std::sync::Arc;

use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).await?);
    println!("Connected to IB Gateway");

    let contract = Contract::stock("AAPL").build();

    println!("Market Data Examples for AAPL\n");
    println!("{}", "=".repeat(50));

    // Example 1: Basic streaming with specific tick types
    example_streaming_with_tick_types(&client, &contract).await?;

    println!("\n{}\n", "=".repeat(50));

    // Example 2: Request a one-time snapshot
    example_snapshot(&client, &contract).await?;

    println!("\n{}\n", "=".repeat(50));

    // Example 3: Demonstrating builder method chaining
    example_builder_chaining(&client, &contract).await?;

    println!("\nAll examples completed!");
    Ok(())
}

async fn example_streaming_with_tick_types(client: &Arc<Client>, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Streaming with specific tick types\n");

    // https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#available-tick-types
    let mut subscription = client
        .market_data(contract)
        .generic_ticks(&["233", "236", "293"]) // RTVolume, Shortable, Trade Count
        .subscribe()
        .await?;

    let mut tick_count = 0;
    while let Some(result) = subscription.next().await {
        match result? {
            TickTypes::Price(price) => {
                println!("Price - Type: {:?}, Value: ${:.2}", price.tick_type, price.price);
                if price.attributes.can_auto_execute {
                    println!("  -> Can auto-execute");
                }
            }
            TickTypes::Size(size) => {
                println!("Size - Type: {:?}, Value: {:.0}", size.tick_type, size.size);
            }
            TickTypes::PriceSize(price_size) => {
                println!(
                    "PriceSize - PriceType: {:?}, Price: ${:.2}, Size: {:.0}",
                    price_size.price_tick_type, price_size.price, price_size.size
                );
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
            subscription.cancel().await;
            break;
        }
    }
    Ok(())
}

async fn example_snapshot(client: &Arc<Client>, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: One-time snapshot\n");

    let mut snapshot_subscription = client.market_data(contract).snapshot().subscribe().await?;

    while let Some(result) = snapshot_subscription.next().await {
        match result? {
            TickTypes::Price(price) => {
                println!("Snapshot Price - Type: {:?}, Value: ${:.2}", price.tick_type, price.price);
            }
            TickTypes::Size(size) => {
                println!("Snapshot Size - Type: {:?}, Value: {:.0}", size.tick_type, size.size);
            }
            TickTypes::SnapshotEnd => {
                println!("\nSnapshot completed!");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn example_builder_chaining(client: &Arc<Client>, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Builder method chaining\n");
    println!("Demonstrating how builder methods can be combined and overridden\n");

    // The builder pattern allows chaining multiple configuration methods.
    // Later method calls override earlier ones for the same setting.
    let mut subscription = client
        .market_data(contract)
        .generic_ticks(&["100", "101", "104"]) // Option Volume, Open Interest, Historical Vol
        .snapshot() // Initially set to snapshot mode
        .streaming() // Override to streaming mode (this takes precedence)
        .subscribe()
        .await?;

    println!("This subscription is in streaming mode (not snapshot)");
    println!("Listening for generic ticks...\n");

    let mut tick_count = 0;
    while let Some(result) = subscription.next().await {
        match result? {
            TickTypes::Generic(generic) => {
                println!("Generic tick - Type: {:?}, Value: {:.2}", generic.tick_type, generic.value);
                tick_count += 1;
                if tick_count >= 5 {
                    println!("\nReceived 5 generic ticks, cancelling...");
                    subscription.cancel().await;
                    break;
                }
            }
            TickTypes::Notice(notice) => {
                println!("Notice - Code: {}, Message: {}", notice.code, notice.message);
            }
            _ => {}
        }
    }
    Ok(())
}
