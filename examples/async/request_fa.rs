//! Request Financial Advisor configuration example (async).
//!
//! Fetches the Financial Advisor groups or aliases XML.
//!
//! ```bash
//! cargo run --example async_request_fa -- groups
//! cargo run --example async_request_fa -- aliases
//! ```

use ibapi::accounts::FaDataType;
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let arg = std::env::args().nth(1).unwrap_or_else(|| "groups".to_string());
    let fa_data_type = match arg.as_str() {
        "groups" => FaDataType::Groups,
        "aliases" => FaDataType::AccountAliases,
        other => {
            eprintln!("unknown FA data type: {other}. expected 'groups' or 'aliases'");
            std::process::exit(2);
        }
    };

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let cfg = client.request_fa(fa_data_type).await.expect("request_fa failed");

    println!("FA config ({:?}):\n{}", cfg.fa_data_type, cfg.xml);
}
