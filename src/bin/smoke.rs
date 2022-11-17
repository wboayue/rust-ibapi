use std::{thread, time};

use anyhow;
use env_logger;
use log::info;

use ibapi::client::BasicClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = BasicClient::connect("odin:4002")?;

    info!("Connected {:?}", client);

    let contract = contracts::stock("MSFT");
    info!("Contract {:?}", contract);

    let result = contracts::contract_details(&mut client, &contract)?;
    info!("details {:?}", result);

    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}
