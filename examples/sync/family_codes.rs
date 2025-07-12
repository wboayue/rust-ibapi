//! Family Codes example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example family_codes
//! ```

use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let family_codes = client.family_codes().expect("request failed");

    for family_code in family_codes {
        println!("account_id: {:4}, family_code: {:4}", family_code.account_id, family_code.family_code)
    }
}
