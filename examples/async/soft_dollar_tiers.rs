//! Soft Dollar Tiers example (async).
//!
//! Lists the soft dollar tiers configured for the connected account.
//!
//! ```bash
//! cargo run --example async_soft_dollar_tiers
//! ```

use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let tiers = client.soft_dollar_tiers().await.expect("soft_dollar_tiers request failed");

    if tiers.is_empty() {
        println!("No soft dollar tiers configured.");
    } else {
        for tier in &tiers {
            println!("{} = {} ({})", tier.name, tier.value, tier.display_name);
        }
    }
}
