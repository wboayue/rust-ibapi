use std::time::Duration;
use std::{thread, time};

use log::{debug, info};

use ibapi::client::IBClient;
use ibapi::contracts::{self, Contract};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = IBClient::connect("odin:4002")?;

    info!("Connected {client:?}");

    let mut contract = Contract::stock("TSLA");
    contract.currency = "USD".to_string();
    debug!("contract template {contract:?}");

    thread::sleep(Duration::from_secs(2));

    let results = contracts::request_contract_details(&mut client, &contract)?;
    for result in &results {
        println!("contract: {result:?}");
    }

    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}
