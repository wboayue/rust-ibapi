//! Sync Account Updates example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example account_updates
//! ```

use ibapi::accounts::AccountUpdate;
use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let account = "DU1234567";

    let subscription = client.account_updates(account).expect("error requesting account updates");
    for update in &subscription {
        println!("{update:?}");

        // stop after full initial update
        if let AccountUpdate::End = update {
            subscription.cancel();
        }
    }
}
