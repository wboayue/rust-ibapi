use std::{thread, time};
use std::time::Duration;

use log::{info, debug};

use ibapi::client::BasicClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = BasicClient::connect("odin:4002")?;

    info!("Connected {:?}", client);

    let contract = contracts::stock("MSFT");
    debug!("Contract {:?}", contract);

    thread::sleep(Duration::from_secs(2));

    let result = contracts::contract_details(&mut client, &contract)?;
    info!("details {:?}", result);

    thread::sleep(time::Duration::from_secs(10));

    Ok(())
}
