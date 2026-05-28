//! User Info example (sync).
//!
//! Fetches white-branding identity information for the logged-in user.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example user_info
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let info = client.user_info().expect("user_info request failed");

    println!("white_branding_id: {}", info.white_branding_id);
}
