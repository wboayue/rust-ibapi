use std::time::Duration;
use std::{thread, time};

use log::{debug, info};

use ibapi::client::BasicClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = BasicClient::connect("odin:4002")?;

    info!("Connected {:?}", client);

    let contract = contracts::stock("TSLA");
    debug!("Contract {:?}", contract);

    thread::sleep(Duration::from_secs(2));

    let results = contracts::contract_details(&mut client, &contract)?;
    for result in &results {
        println!(
            "symbol: {:?}, exchange: {:?}, currency: {:?}",
            result.contract.symbol, result.contract.exchange, result.contract.currency
        );
    }

    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}
