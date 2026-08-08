//! Wsh Event Data By Contract example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example wsh_event_data_by_contract
//! ```

use ibapi::client::blocking::Client;

// This example demonstrates requesting Wall Street Horizon event data by contract ID.

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract_id = 76792991; // TSLA

    // Optional narrowing lives on the builder: .starting(), .ending(),
    // .limit(), .auto_fill(). Unset means TWS decides.
    let event_data = client
        .wsh_event_data_by_contract(contract_id)
        .fetch()
        .expect("request wsh event data failed");

    println!("{}", event_data.data_json);
}
