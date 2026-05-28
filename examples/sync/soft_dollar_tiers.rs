//! Soft Dollar Tiers example (sync).
//!
//! Lists the soft dollar tiers configured for the connected account.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example soft_dollar_tiers
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let tiers = client.soft_dollar_tiers().expect("soft_dollar_tiers request failed");

    if tiers.is_empty() {
        println!("No soft dollar tiers configured.");
    } else {
        for tier in &tiers {
            println!("{} = {} ({})", tier.name, tier.value, tier.display_name);
        }
    }
}
