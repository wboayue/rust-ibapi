//! Update Config example (async).
//!
//! Edits the TWS/Gateway smart-routing configuration. If the gateway returns
//! warnings, the edit is not applied — re-submit with each warning passed to
//! `.accept_warning(...)`.
//!
//! ```bash
//! cargo run --example async_update_config
//! ```

use ibapi::config::{OrdersConfig, OrdersSmartRouting};
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let response = client
        .update_config()
        .orders(OrdersConfig {
            smart_routing: Some(OrdersSmartRouting {
                seek_price_improvement: Some(true),
                ..Default::default()
            }),
        })
        .submit()
        .await
        .expect("update config failed");

    println!("{response:#?}");
}
