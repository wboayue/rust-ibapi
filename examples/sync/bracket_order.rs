//! Bracket Order example
//!
//! Submits a bracket order using the fluent builder. `bracket()` chains entry, take-profit,
//! and stop-loss legs; `submit_all()` allocates contiguous order ids and transmits the
//! children only after the parent is in place.
//!
//! For status / execution monitoring across all submitted orders, see
//! `examples/sync/submit_order.rs`, which sets up `client.order_update_stream()` in a
//! background thread.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example bracket_order
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

fn main() {
    env_logger::init();
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

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
}
