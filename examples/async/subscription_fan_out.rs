#![allow(clippy::uninlined_format_args)]
//! Async subscription fan-out example
//!
//! Clones one market data subscription so two tasks read the same ticks.
//! Each clone has its own read position and sees every tick; TWS sees one
//! request. A clone dropped early ends only that reader: the request is
//! cancelled when the last clone is dropped. Here the printer stops after a
//! few ticks while the counter keeps reading until its deadline.
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

    let deadline = Instant::now() + Duration::from_secs(10);
    let printer = tokio::spawn(read("printer", subscription.clone(), deadline, Some(5)));
    let counter = tokio::spawn(read("counter", subscription.clone(), deadline, None));
    // Only the readers' clones are left; the last one dropped cancels the request at TWS.
    drop(subscription);

    println!("printer read {} ticks", printer.await?);
    println!("counter read {} ticks", counter.await?);
    Ok(())
}

/// Reads up to `limit` ticks (printing them when limited), stopping at
/// `deadline` even if no tick arrives. Notices are logged at `warn`
/// (run with `RUST_LOG=warn` to see them).
async fn read(name: &'static str, subscription: Subscription<TickTypes>, deadline: Instant, limit: Option<usize>) -> usize {
    let mut ticks = subscription.filter_data();
    let mut count = 0;
    while limit.is_none_or(|limit| count < limit) {
        match timeout_at(deadline, ticks.next()).await {
            Ok(Some(Ok(tick))) => {
                count += 1;
                if limit.is_some() {
                    println!("[{name}] {tick:?}");
                }
            }
            Ok(Some(Err(e))) => {
                eprintln!("[{name}] error: {e}");
                break;
            }
            Ok(None) | Err(_) => break,
        }
    }
    count
}
