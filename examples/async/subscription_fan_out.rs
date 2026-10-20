#![allow(clippy::uninlined_format_args)]
//! Async subscription fan-out example
//!
//! Clones one market data subscription so two tasks read the same ticks.
//! Each clone has its own read position and sees every tick; TWS sees one
//! request. Clones share one cancel: cancelling or dropping any clone ends
//! the stream for all of them.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example async_subscription_fan_out
//! ```

use std::time::Duration;

use ibapi::prelude::*;
use ibapi::subscriptions::Subscription;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to IB Gateway");

    let contract = Contract::stock("AAPL").build();
    let subscription = client.market_data(&contract).subscribe().await?;

    let printer = tokio::spawn(consume("printer", subscription.clone()));
    let counter = tokio::spawn(consume("counter", subscription.clone()));

    // Keep the original to end the stream: cancel reaches every clone.
    tokio::time::sleep(Duration::from_secs(10)).await;
    subscription.cancel().await;

    println!("printer read {} ticks", printer.await?);
    println!("counter read {} ticks", counter.await?);
    Ok(())
}

async fn consume(name: &'static str, subscription: Subscription<TickTypes>) -> usize {
    let mut ticks = subscription.filter_data();
    let mut count = 0;
    while let Some(tick) = ticks.next().await {
        match tick {
            Ok(tick) => {
                count += 1;
                if name == "printer" {
                    println!("[{name}] {tick:?}");
                }
            }
            Err(Error::Cancelled) => break,
            Err(e) => {
                eprintln!("[{name}] error: {e}");
                break;
            }
        }
    }
    count
}
