//! Historical News example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example historical_news
//! ```

use time::macros::datetime;

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract_id = 76792991; // TSLA
    let provider_codes = vec!["BRFG", "DJ-N", "DJ-RT"];
    let start_time = datetime!(2024-10-27 0:00 UTC);
    let end_time = datetime!(2024-10-29 0:00 UTC);
    let total_results = 10;

    let articles = client
        .historical_news(contract_id, &provider_codes, start_time, end_time, total_results)
        .expect("request historical news failed");

    for article in &articles {
        println!("{}: {}", article.article_id, article.headline);
    }
}
