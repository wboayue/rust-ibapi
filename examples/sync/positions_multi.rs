//! Positions Multi example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example positions_multi
//! ```

use std::env;

use ibapi::Client;

pub fn main() {
    env_logger::init();

    let account = env::var("IBKR_ACCOUNT").expect("Please set IBKR_ACCOUNT environment variable to an account ID");

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let subscription = client.positions_multi(Some(&account), None).expect("error requesting positions by model");
    for position in subscription.iter() {
        println!("{position:?}")
    }
}
