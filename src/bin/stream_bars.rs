use std::thread;
use std::time::Duration;

use clap::{arg, ArgMatches, Command};

use ibapi::contracts::Contract;
use ibapi::market_data::{realtime, BarSize, WhatToShow};
use ibapi::Client;

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

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    let contract = extract_contract(&matches).expect("error parsing --stock or --future");

    println!("connection_string: {connection_string:?}");
    println!("contract: {contract:?}");

    let client = Client::connect("odin:4002")?;

    let bars = realtime::realtime_bars(&client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;

    for (i, bar) in bars.enumerate().take(60) {
        println!("bar: {i:?} {bar:?}");
    }

    thread::sleep(Duration::from_secs(5));

    Ok(())
}

fn extract_contract(matches: &ArgMatches) -> Option<Contract> {
    if let Some(symbol) = matches.get_one::<String>("stock") {
        Some(Contract::stock(&symbol.to_uppercase()))
    } else {
        matches
            .get_one::<String>("futures")
            .map(|symbol| Contract::futures(&symbol.to_uppercase()))
    }
}
