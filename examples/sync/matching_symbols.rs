//! Matching Symbols example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example matching_symbols
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let pattern = "TSLA";
    let results = client.matching_symbols(pattern).unwrap();
    for result in results {
        println!("contract: {result:?}");
    }
}
