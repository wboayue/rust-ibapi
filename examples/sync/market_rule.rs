//! Market Rule example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example market_rule
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let market_rule_id = 12;
    let results = client.market_rule(market_rule_id).unwrap();

    println!("rule: {results:?}");
}
