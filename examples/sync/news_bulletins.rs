//! News Bulletins example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example news_bulletins
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let news_bulletins = client.news_bulletins(true).expect("request news providers failed");
    for news_bulletin in &news_bulletins {
        println!("news bulletin {news_bulletin:?}");
    }
}
