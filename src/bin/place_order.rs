use std::time::Duration;
use std::{thread, time};

use clap::{arg, Command, ArgMatches};
use log::{debug, info};

use ibapi::client::IBClient;
use ibapi::contracts::{self, Contract};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("place_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Submits order to broker")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--stock <SYMBOL>).required(true))
        .arg(arg!(--buy <QUANTITY>))
        .arg(arg!(--sell <QUANTITY>))
        .get_matches();

    let connection_string = matches
        .get_one::<String>("connection_string")
        .expect("connection_string is required");
    let stock_symbol = matches
        .get_one::<String>("stock")
        .expect("stock symbol is required");

    if let Some((action, quantity)) = get_order(&matches) {
        println!("action: {action}, quantity: {quantity}");
    }

    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

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

fn get_order(matches: &ArgMatches) -> Option<(String, f64)> {
    if let Some(quantity) = matches.get_one::<f64>("buy") {
        Some(("BUY".to_string(), *quantity))
    } else if let Some(quantity) = matches.get_one::<f64>("sell") {
        Some(("SELL".to_string(), *quantity))   
    } else {
        None
    }
}