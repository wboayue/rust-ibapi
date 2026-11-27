//! Positions Multi example
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example positions_multi
//! ```

use ibapi::client::blocking::Client;
use std::env;

pub fn main() {
    env_logger::init();

    let account = env::var("IBKR_ACCOUNT").expect("Please set IBKR_ACCOUNT environment variable to an account ID");

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let subscription = client
        .positions_multi()
        .account(account)
        .subscribe()
        .expect("error requesting positions by model");
    for position in subscription.iter_data() {
        match position {
            Ok(position) => println!("{position:?}"),
            Err(e) => {
                eprintln!("error: {e:?}");
                break;
            }
        }
    }
}
