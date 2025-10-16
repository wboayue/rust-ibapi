//! Contract Details example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example contract_details
//! ```

use ibapi::client::blocking::Client;
use ibapi::prelude::*;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    println!("server_version: {}", client.server_version());
    println!("connection_time: {:?}", client.connection_time());
    println!("next_order_id: {}", client.next_order_id());

    let contract = Contract::stock("MSFT").build();

    let results = client.contract_details(&contract)?;
    for contract in results {
        println!("contract: {contract:?}");
    }

    Ok(())
}
