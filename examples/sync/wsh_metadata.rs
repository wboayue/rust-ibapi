//! Wsh Metadata example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example wsh_metadata
//! ```

use ibapi::client::blocking::Client;

// This example demonstrates requesting Wall Street Horizon metadata.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let metadata = client.wsh_metadata().expect("request wsh metadata failed");
    println!("{}", metadata.data_json);
}
