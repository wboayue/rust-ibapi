//! Market Data example
//!
//! This example demonstrates how to subscribe to market data using the builder API.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_data
//! ```

use ibapi::prelude::*;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::stock("AAPL").build();

    println!("Market Data Examples for AAPL\n");
    println!("{}", "=".repeat(50));

    // Example 1: Basic streaming with specific tick types
    example_streaming_with_tick_types(&client, &contract);

    println!("\n{}\n", "=".repeat(50));

    // Example 2: Request a one-time snapshot
    example_snapshot(&client, &contract);

    println!("\n{}\n", "=".repeat(50));

    // Example 3: Demonstrating builder method chaining
    example_builder_chaining(&client, &contract);

    println!("\nAll examples completed!");
}

fn example_streaming_with_tick_types(client: &Client, contract: &Contract) {
    println!("Example 1: Streaming with specific tick types\n");

    // https://www.interactivebrokers.com/campus/ibkr-api-page/twsapi-doc/#available-tick-types
    let subscription = client
        .market_data(contract)
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
            subscription.cancel();
            break;
        }
    }
}

fn example_snapshot(client: &Client, contract: &Contract) {
    println!("Example 2: One-time snapshot\n");

    let snapshot_subscription = client.market_data(contract).snapshot().subscribe().expect("Failed to request snapshot");

    for tick in &snapshot_subscription {
        match tick {
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
}

fn example_builder_chaining(client: &Client, contract: &Contract) {
    println!("Example 3: Builder method chaining\n");
    println!("Demonstrating how builder methods can be combined and overridden\n");

    // The builder pattern allows chaining multiple configuration methods.
    // Later method calls override earlier ones for the same setting.
    let subscription = client
        .market_data(contract)
        .generic_ticks(&["100", "101", "104"]) // Option Volume, Open Interest, Historical Vol
        .snapshot() // Initially set to snapshot mode
        .streaming() // Override to streaming mode (this takes precedence)
        .subscribe()
        .expect("Failed to subscribe");

    println!("This subscription is in streaming mode (not snapshot)");
    println!("Listening for generic ticks...\n");

    let mut tick_count = 0;
    for tick in &subscription {
        match tick {
            TickTypes::Generic(generic) => {
                println!("Generic tick - Type: {:?}, Value: {:.2}", generic.tick_type, generic.value);
                tick_count += 1;
                if tick_count >= 5 {
                    println!("\nReceived 5 generic ticks, cancelling...");
                    subscription.cancel();
                    break;
                }
            }
            TickTypes::Notice(notice) => {
                println!("Notice - Code: {}, Message: {}", notice.code, notice.message);
            }
            _ => {}
        }
    }
}
