//! Display group events example
//!
//! Display Groups are a TWS-only feature (not available in IB Gateway).
//! They allow organizing contracts into color-coded groups in the TWS UI.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example group_events
//! ```
//!
//! Make sure TWS is running with API connections enabled

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    println!("Connecting to TWS...");

    // Display Groups require TWS (not IB Gateway)
    let client = Client::connect("127.0.0.1:7497", 100).expect("connection failed");
    println!("Connected successfully!");

    println!("\nSubscribing to group events for group 1...");
    // 1 corresponds to "Group 1" in TWS (Red)
    let subscription = client.subscribe_to_group_events(1).expect("subscription failed");

    println!("Listening for events. Change the contract in TWS Group 1 (Red) to see updates.");

    for event in &subscription {
        println!("Received group event: {:?}", event);
    }

    if let Some(err) = subscription.error() {
        eprintln!("Subscription error: {err}");
    }
}
