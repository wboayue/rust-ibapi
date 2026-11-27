//! Sync Account Updates Multi example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example account_updates_multi
//! ```

use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let subscription = client
        .account_updates_multi(LedgerOnly::No)
        .account("DU1234567")
        .subscribe()
        .expect("error requesting account updates multi");
    for update in subscription.iter_data() {
        let update = match update {
            Ok(update) => update,
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        };
        println!("{update:?}");

        // stop after full initial update
        if let AccountUpdateMulti::End = update {
            subscription.cancel();
        }
    }
}
