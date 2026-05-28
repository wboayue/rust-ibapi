//! User Info example (async).
//!
//! Fetches white-branding identity information for the logged-in user.
//!
//! ```bash
//! cargo run --example async_user_info
//! ```

use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let info = client.user_info().await.expect("user_info request failed");

    println!("white_branding_id: {}", info.white_branding_id);
}
