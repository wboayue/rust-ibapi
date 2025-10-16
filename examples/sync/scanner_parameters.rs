//! Scanner Parameters example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example scanner_parameters
//! ```

use ibapi::client::blocking::Client;

// This example demonstrates requesting scanner parameters from the TWS.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let parameters = client.scanner_parameters().expect("request scanner parameters failed");
    println!("{parameters:?}");
}
