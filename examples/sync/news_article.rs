//! News Article example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example news_article
//! ```

use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // See historical_news example for getting provider_code and article_id
    let provider_code = "DJ-N";
    let article_id = "DJ-N$1915168d";

    let article = client.news_article(provider_code, article_id).expect("request news article failed");
    println!("{article:?}");
}
