//! Replace Financial Advisor configuration example (async).
//!
//! Replaces the FA groups or aliases XML on the server. Reads the
//! replacement XML from stdin.
//!
//! ```bash
//! cat new_groups.xml | cargo run --example async_replace_fa -- groups
//! ```

use ibapi::accounts::FaDataType;
use ibapi::prelude::*;
use std::io::Read;

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

    let mut xml = String::new();
    std::io::stdin().read_to_string(&mut xml).expect("read stdin failed");

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let result = client.replace_fa(fa_data_type, &xml).await.expect("replace_fa failed");

    println!("{}", result.text);
}
