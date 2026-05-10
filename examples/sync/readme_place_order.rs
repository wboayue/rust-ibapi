//! Readme Place Order example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example readme_place_order
//! ```

use ibapi::client::blocking::Client;
use ibapi::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";
    let client = Arc::new(Client::connect(connection_url, 100).expect("connection to TWS failed!"));

    // Start a background monitor for all order updates before submitting.
    let monitor_client = client.clone();
    let _monitor_handle = thread::spawn(move || {
        let stream = monitor_client.order_update_stream().expect("failed to create order update stream");
        for update in stream.iter_data() {
            let update = match update {
                Ok(update) => update,
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
            };
            if let OrderUpdate::ExecutionData(data) = update {
                println!("{} {} shares of {}", data.execution.side, data.execution.shares, data.contract.symbol);
            } else {
                println!("{update:?}");
            }
        }
    });

    // Give the monitor a moment to start.
    thread::sleep(Duration::from_millis(100));

    let contract = Contract::stock("AAPL").build();

    // Submit a market order to buy 100 shares using the fluent builder.
    // `submit()` allocates an order id internally and uses fire-and-forget delivery;
    // status flows through `order_update_stream` above.
    let order_id = client.order(&contract).buy(100).market().submit().expect("place order request failed!");
    println!("Submitted order: {order_id}");

    // Wait for executions to flow through the monitor; the monitor thread ends when the process exits.
    thread::sleep(Duration::from_secs(5));
}
