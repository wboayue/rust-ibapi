//! Display group events example
//!
//! Display Groups are a TWS-only feature (not available in IB Gateway).
//! They allow organizing contracts into color-coded groups in the TWS UI.
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example group_events
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

    // Visible groups, most used first; 1 is "Group 1" in TWS (Red).
    let groups = client.display_groups().expect("display groups request failed");
    println!("Visible display groups: {groups:?}");
    let Some(&group_id) = groups.first() else {
        eprintln!("No display groups visible in TWS");
        return;
    };

    println!("\nSubscribing to group events for group {group_id}...");
    let subscription = client.subscribe_to_group_events(group_id).expect("subscription failed");

    println!("Listening for events. Change the contract in TWS group {group_id} to see updates.");

    for event in subscription.iter_data() {
        match event {
            Ok(event) => println!("Received group event: {:?}", event),
            Err(err) => {
                eprintln!("Subscription error: {err}");
                break;
            }
        }
    }
}
