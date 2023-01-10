use std::thread;
use std::time::Duration;

use log::info;

use ibapi::client::IBClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = IBClient::connect("odin:4002")?;

    info!("Connected {:?}", client);

    thread::sleep(Duration::from_secs(2));

    let results = contracts::request_matching_symbols(&mut client, "microsoft")?;
    for result in &results {
        println!("contract: {:?}", result);
    }

    Ok(())
}
