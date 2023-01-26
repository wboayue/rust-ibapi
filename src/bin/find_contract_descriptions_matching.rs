use std::thread;
use std::time::Duration;

use clap::{arg, Command};
use log::info;

use ibapi::client::IBClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("find_contract_descriptions_matching")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Finds contract descriptions matching pattern")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--pattern <VALUE>).required(true))
        .get_matches();

    let connection_string = matches
        .get_one::<String>("connection_string")
        .expect("connection_string is required");
    let pattern = matches
        .get_one::<String>("pattern")
        .expect("pattern is required");

    let mut client = IBClient::connect(connection_string)?;

    info!("connected {:?}, using: {:?}", client, connection_string);

    thread::sleep(Duration::from_secs(2));

    let results = contracts::request_matching_symbols(&mut client, pattern)?;
    for result in &results {
        println!("contract: {:?}", result);
    }

    Ok(())
}
