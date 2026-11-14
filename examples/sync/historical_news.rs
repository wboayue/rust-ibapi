//! Historical News example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example historical_news
//! ```

use time::macros::datetime;

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract_id = 76792991; // TSLA
    let provider_codes = ["BRFG", "DJ-N", "DJ-RT"];

    // The newest 10 headlines before the date; `.since(..)` instead returns those after it.
    let articles = client
        .historical_news(contract_id, &provider_codes)
        .before(datetime!(2026-11-01 0:00 UTC))
        .limit(10)
        .subscribe()
        .expect("request historical news failed");

    for article in articles.iter_data() {
        match article {
            Ok(article) => println!("{}: {}", article.article_id, article.headline),
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }
}
