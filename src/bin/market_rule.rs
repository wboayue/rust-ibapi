use clap::{arg, Command};
use log::info;

use ibapi::client::IBClient;
use ibapi::contracts;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("market_rule")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Finds contract descriptions matching pattern")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--market_rule_id <VALUE>).required(true).value_parser(clap::value_parser!(i32)))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let market_rule_id = matches.get_one::<i32>("market_rule_id").expect("market rule id is required");

    let mut client = IBClient::connect(connection_string)?;

    info!("connected {client:?}, using: {connection_string}");

    let results = contracts::market_rule(&mut client, *market_rule_id)?;
    println!("rule: {results:?}");

    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}
