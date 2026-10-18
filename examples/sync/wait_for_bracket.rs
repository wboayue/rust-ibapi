//! Example: wait for every leg of a bracket order to finish.
//!
//! A bracket is done when each leg has reached a final state: the entry was cancelled
//! before any fill (TWS cancels both exits) or rejected (both exits go `Inactive`), or the entry filled
//! and one exit filled (TWS cancels the other). Cancelling one exit also cancels the
//! other but leaves the entry working. Calling `wait_for_fill` on each leg with one
//! shared deadline covers every case: a leg that already finished returns at once from
//! the status cache, so the total wait is the slowest leg.
//!
//! The entry is a buy limit far below the market, so the first wait times out with the
//! bracket still working. Cancelling the entry cancels the exits, and the second
//! wait reports all three legs ended.
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example wait_for_bracket
//! ```

use std::time::{Duration, Instant};

use ibapi::client::blocking::Client;
use ibapi::orders::BracketOrderIds;
use ibapi::prelude::*;

/// Waits on the entry, take-profit and stop-loss legs, in that order, until `timeout` runs out.
fn wait_for_bracket(client: &Client, ids: &BracketOrderIds, timeout: Duration) -> Result<[OrderOutcome; 3], Error> {
    let deadline = Instant::now() + timeout;
    let remaining = || deadline.saturating_duration_since(Instant::now());
    Ok([
        client.wait_for_fill(ids.parent, remaining())?,
        client.wait_for_fill(ids.take_profit, remaining())?,
        client.wait_for_fill(ids.stop_loss, remaining())?,
    ])
}

fn report(ids: &BracketOrderIds, outcomes: &[OrderOutcome; 3]) {
    let legs = [("entry", ids.parent), ("take profit", ids.take_profit), ("stop loss", ids.stop_loss)];
    for ((leg, order_id), outcome) in legs.iter().zip(outcomes) {
        let state = match outcome {
            OrderOutcome::Filled(status) => format!("filled {} @ {:?}", status.filled, status.average_fill_price),
            OrderOutcome::Ended(status) => format!("ended: {} after {} filled", status.status, status.filled),
            OrderOutcome::TimedOut(Some(status)) => format!("timed out, still {}", status.status),
            OrderOutcome::TimedOut(None) => "timed out, no status seen".to_string(),
            OrderOutcome::Rejected(notice) => format!("rejected: {notice}"),
        };
        println!("  {leg} {order_id}: {state}");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;
    let contract = Contract::stock("AAPL").build();

    let ids = client
        .order(&contract)
        .buy(1)
        .bracket()
        .entry_limit(1.00)
        .take_profit(2.00)
        .stop_loss(0.50)
        .submit_all()?;
    println!("{ids} submitted");
    report(&ids, &wait_for_bracket(&client, &ids, Duration::from_secs(5))?);

    // Held so its route stays open; the waits below get their own copy of each status.
    let _cancellation = client.cancel_order(ids.parent, "")?;
    println!("entry cancelled");
    report(&ids, &wait_for_bracket(&client, &ids, Duration::from_secs(10))?);

    Ok(())
}
