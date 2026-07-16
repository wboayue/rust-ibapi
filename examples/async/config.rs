//! Config example (async).
//!
//! Reads the TWS/Gateway configuration (API settings, order precautions,
//! smart-routing, lock-and-exit) the gateway is currently running with.
//!
//! ```bash
//! cargo run --example async_config
//! ```

use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let config = client.config().await.expect("config request failed");

    println!("{config:#?}");
}
