//! Contract Details example
//!
//! # Usage
//!
//! ```bash
//! cargo run --example contract_details
//! ```

use ibapi::contracts::Contract;
use ibapi::Client;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    println!("server_version: {}", client.server_version());
    println!("connection_time: {:?}", client.connection_time());
    println!("next_order_id: {}", client.next_order_id());

    let contract = Contract::stock("MSFT");

    let results = client.contract_details(&contract)?;
    for contract in results {
        println!("contract: {contract:?}");
    }

    Ok(())
}
