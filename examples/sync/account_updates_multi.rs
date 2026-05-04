//! Sync Account Updates Multi example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example account_updates_multi
//! ```

use ibapi::accounts::{types::AccountId, AccountUpdateMulti};
use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let account = Some(AccountId("DU1234567".to_string()));

    let subscription = client
        .account_updates_multi(account.as_ref(), None)
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
