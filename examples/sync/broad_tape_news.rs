//! Broad Tape News example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example broad_tape_news
//! ```

use ibapi::Client;

// This example demonstrates how live news for a contract can be requested.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let news_source = "BRFG";

    let subscription = client.broad_tape_news(news_source).expect("request broad tape news failed");
    for article in subscription {
        println!("{article:?}");
    }
}
