use std::thread;
use std::time::Duration;

use clap::{arg, ArgMatches, Command};

use ibapi::client::IBClient;
use ibapi::contracts::Contract;
use ibapi::market_data::{streaming, BarSize, WhatToShow};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("stream_bars")
        .version("1.0")
        .author("Wil Boayue <wil@wsbsolutions.com")
        .about("Streams realtime bars")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--stock <SYMBOL>))
        .arg(arg!(--futures <SYMBOL>))
        .get_matches();

    let connection_string = matches
        .get_one::<String>("connection_string")
        .expect("connection_string is required");
    let contract = extract_contract(&matches).expect("error parsing --stock or --future");

    println!("connection_string: {connection_string:?}");
    println!("contract: {contract:?}");

    let mut client = IBClient::connect("odin:4002")?;

    let bars = streaming::realtime_bars(
        &mut client,
        &contract,
        &BarSize::Secs5,
        &WhatToShow::Trades,
        false,
    )?;
    for (i, bar) in bars.enumerate() {
        println!("bar: {i:?} {bar:?}");

        if i > 60 {
            break;
        }
    }

    // let mut contract = Contract::stock(stock_symbol);
    // contract.currency = "USD".to_string();
    // debug!("contract template: {contract:?}");

    thread::sleep(Duration::from_secs(5));

    Ok(())
}

fn extract_contract(matches: &ArgMatches) -> Option<Contract> {
    if matches.contains_id("stock") {
        let symbol = matches
            .get_one::<String>("stock")
            .expect("error parsing stock symbol");

        Some(Contract::stock(&symbol.to_uppercase()))
    } else {
        let symbol = matches
            .get_one::<String>("futures")
            .expect("error parsing futures symbol");

        Some(Contract::futures(&symbol.to_uppercase()))
    }
}
