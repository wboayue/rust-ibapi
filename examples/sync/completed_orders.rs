//! Completed Orders example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example completed_orders
//! ```

use ibapi::Client;

// This example demonstrates how to request completed orders.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let subscription = client.completed_orders(false).expect("get completed orders failed");
    for order in subscription {
        println!("{order:?}");
    }
}
