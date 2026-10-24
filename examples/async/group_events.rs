#![allow(clippy::uninlined_format_args)]
//! Example of subscribing to TWS display group events asynchronously
//!
//! Display Groups are a TWS-only feature (not available in IB Gateway).
//! They allow organizing contracts into color-coded groups in the TWS UI.
//!
//! To run this example:
//! ```bash
//! cargo run --example async_group_events
//! ```
//!
//! Make sure TWS is running with API connections enabled

use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting to TWS...");

    // Display Groups require TWS (not IB Gateway)
    let client = Client::connect("127.0.0.1:7497", 100).await?;
    println!("Connected successfully!");

    // Visible groups, most used first; 1 is "Group 1" in TWS (Red).
    let groups = client.display_groups().await?;
    println!("Visible display groups: {:?}", groups);
    let Some(&group_id) = groups.first() else {
        eprintln!("No display groups visible in TWS");
        return Ok(());
    };

    println!("\nSubscribing to group events for group {}...", group_id);
    let mut subscription = client.subscribe_to_group_events(group_id).await?;

    println!("Listening for events. Change the contract in TWS group {} to see updates.", group_id);

    // DisplayGroupSubscription derefs to Subscription; deref-then-borrow before filter_data.
    let inner = &mut *subscription;
    while let Some(result) = inner.filter_data().next().await {
        match result {
            Ok(event) => {
                println!("Received group event: {:?}", event);
                println!("  Contract info: {}", event.contract_info);
            }
            Err(e) => {
                eprintln!("Error receiving group event: {e:?}");
                break;
            }
        }
    }

    Ok(())
}
