use std::{thread, time};

use anyhow;
use env_logger;
use log::info;

use ibapi::client::BasicClient;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let client = BasicClient::connect("odin:4002")?;

    info!("Connected {:?}", client);

    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}
