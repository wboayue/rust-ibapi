//! Server time example
//!
//! This example demonstrates how to retrieve the current server time from TWS/IB Gateway.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example server_time
//! ```

use ibapi::client::blocking::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    let server_time = client.server_time().expect("error requesting server time");
    println!("server time: {server_time:?}");
}
