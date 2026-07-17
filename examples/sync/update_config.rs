//! Update Config example (sync).
//!
//! Edits the TWS/Gateway smart-routing configuration. If the gateway returns
//! warnings, the edit is not applied — re-submit with each warning passed to
//! `.accept_warning(...)`.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example update_config
//! ```

use ibapi::client::blocking::Client;
use ibapi::config::{OrdersConfig, OrdersSmartRouting};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let response = client
        .update_config()
        .orders(OrdersConfig {
            smart_routing: Some(OrdersSmartRouting {
                seek_price_improvement: Some(true),
                ..Default::default()
            }),
        })
        .submit()
        .expect("update config failed");

    println!("{response:#?}");
}
