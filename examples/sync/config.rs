//! Config example (sync).
//!
//! Reads the TWS/Gateway configuration (API settings, order precautions,
//! smart-routing, lock-and-exit) the gateway is currently running with.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example config
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let config = client.config().expect("config request failed");

    println!("{config:#?}");
}
