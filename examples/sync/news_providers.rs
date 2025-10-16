//! News Providers example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example news_providers
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let news_providers = client.news_providers().expect("request news providers failed");
    for news_provider in &news_providers {
        println!("news provider {news_provider:?}");
    }
}
