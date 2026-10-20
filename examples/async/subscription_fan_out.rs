#![allow(clippy::uninlined_format_args)]
//! Async subscription fan-out example
//!
//! Clones one market data subscription so two tasks read the same ticks.
//! Each clone has its own read position and sees every tick; TWS sees one
//! request. Clones share one cancel: cancelling or dropping any clone ends
//! the request for all of them, so both readers here stop at one shared
//! deadline.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example async_subscription_fan_out
//! ```

use std::time::Duration;

use ibapi::prelude::*;
use ibapi::subscriptions::Subscription;
use tokio::time::{timeout_at, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("Connected to IB Gateway");

    let contract = Contract::stock("AAPL").build();
    let subscription = client.market_data(&contract).subscribe().await?;

    // Both readers stop at the same deadline, even if no tick arrives.
    let deadline = Instant::now() + Duration::from_secs(10);
    let printer = tokio::spawn(consume("printer", subscription.clone(), deadline, true));
    let counter = tokio::spawn(consume("counter", subscription.clone(), deadline, false));

    // The first reader to return drops its clone, which cancels the request at TWS.
    println!("printer read {} ticks", printer.await?);
    println!("counter read {} ticks", counter.await?);
    Ok(())
}

/// Reads ticks until `deadline`. Notices are logged at `warn`
/// (run with `RUST_LOG=warn` to see them).
async fn consume(name: &'static str, subscription: Subscription<TickTypes>, deadline: Instant, print: bool) -> usize {
    let mut ticks = subscription.filter_data();
    let mut count = 0;
    while let Ok(Some(tick)) = timeout_at(deadline, ticks.next()).await {
        match tick {
            Ok(tick) => {
                count += 1;
                if print {
                    println!("[{name}] {tick:?}");
                }
            }
            Err(e) => {
                eprintln!("[{name}] error: {e}");
                break;
            }
        }
    }
    count
}
