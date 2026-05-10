//! Bracket Order example
//!
//! Submits a bracket order using the fluent builder. `bracket()` chains entry, take-profit,
//! and stop-loss legs; `submit_all()` allocates contiguous order ids and transmits the
//! children only after the parent is in place.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example bracket_order
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::orders::OrderUpdate;

fn main() {
    env_logger::init();
    let client = Arc::new(Client::connect("127.0.0.1:4002", 100).expect("connection failed"));

    // Background monitor for status / executions on all three legs.
    let monitor_client = client.clone();
    let _monitor = thread::spawn(move || {
        let stream = match monitor_client.order_update_stream() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to open order update stream: {e}");
                return;
            }
        };
        for update in stream.iter_data() {
            match update {
                Ok(OrderUpdate::OrderStatus(s)) => {
                    println!("order {} status: {}", s.order_id, s.status);
                }
                Ok(other) => println!("{other:?}"),
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(100));

    let contract = Contract::stock("AAPL").build();

    let bracket_ids = client
        .order(&contract)
        .buy(100)
        .bracket()
        .entry_limit(220.00)
        .take_profit(230.00)
        .stop_loss(210.00)
        .submit_all()
        .expect("bracket order submission failed");

    println!("Bracket placed: {bracket_ids}");

    thread::sleep(Duration::from_secs(10));
}
