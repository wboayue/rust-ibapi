//! Options Purchase example
//!
//! Submits two market buys on an AAPL call option via the fluent builder.
//!
//! For status / execution monitoring, see `examples/sync/submit_order.rs`, which
//! sets up `client.order_update_stream()` in a background thread.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example options_purchase
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract = Contract::call("AAPL").strike(180.0).expires_on(2025, 2, 21).build();
    println!("contract: {contract:?}");

    // Submit two 5-contract market buys via the fluent path.
    for i in 1..=2 {
        match client.order(&contract).buy(5).market().submit() {
            Ok(id) => println!("Submitted order #{i}: {id}"),
            Err(e) => eprintln!("Submit #{i} failed: {e}"),
        }
    }
}
