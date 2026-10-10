//! Example: wait for an order to finish with `wait_for_fill`.
//!
//! 1. A 1-share market order, waited on until it fills.
//! 2. A buy limit far below the market: the wait times out with the order still
//!    working, the order is cancelled, and a second wait reports it ended.
//!
//! Run against a paper account during market hours.

use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn report(outcome: &OrderOutcome) {
    match outcome {
        OrderOutcome::Filled(status) => println!("filled {} @ {:?}", status.filled, status.average_fill_price),
        OrderOutcome::Ended(status) => println!("ended: {} after {} filled", status.status, status.filled),
        OrderOutcome::TimedOut(Some(status)) => println!(
            "timed out, still {}: {}/{} filled",
            status.status,
            status.filled,
            status.filled + status.remaining
        ),
        OrderOutcome::TimedOut(None) => println!("timed out, no status seen"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;
    let contract = Contract::stock("AAPL").build();

    let order_id = client.order(&contract).buy(1).market().submit()?;
    println!("market order {order_id} submitted");
    report(&client.wait_for_fill(order_id, Duration::from_secs(30))?);

    let order_id = client.order(&contract).buy(1).limit(1.0).submit()?;
    println!("limit order {order_id} submitted");
    report(&client.wait_for_fill(order_id, Duration::from_secs(3))?);

    // Held so its route stays open; the wait below gets its own copy of each status.
    let _cancellation = client.cancel_order(order_id, "")?;
    report(&client.wait_for_fill(order_id, Duration::from_secs(10))?);

    Ok(())
}
