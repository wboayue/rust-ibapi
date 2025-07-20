//! Managed accounts example
//!
//! This example demonstrates how to retrieve the list of managed accounts.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example managed_accounts
//! ```

use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 101).expect("connection failed");

    let accounts = client.managed_accounts().expect("error requesting managed accounts");
    println!("managed accounts: {accounts:?}")
}
