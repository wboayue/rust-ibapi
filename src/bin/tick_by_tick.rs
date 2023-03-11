use std::thread;
use std::time::Duration;

use clap::{arg, ArgMatches, Command};

use ibapi::client::IBClient;
use ibapi::contracts::Contract;
use ibapi::market_data::{realtime, BarSize, WhatToShow};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("tick_by_tick")
        .version("1.0")
        .author("Wil Boayue <wil@wsbsolutions.com")
        .about("Streams tick by tick data")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--last <SYMBOL>))
        .arg(arg!(--all_last <SYMBOL>))
        .arg(arg!(--bid_ask <SYMBOL>))
        .arg(arg!(--mid_point <SYMBOL>))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    println!("connection_string: {connection_string}");

    let client = IBClient::connect(&connection_string)?;

    if let Some(symbol) = matches.get_one::<String>("last") {
        stream_last(&client, &symbol.to_uppercase());
    }

    if let Some(symbol) = matches.get_one::<String>("all_last") {
        stream_all_last(&client, &symbol.to_uppercase());
    }

    if let Some(symbol) = matches.get_one::<String>("bid_ask") {
        stream_bid_ask(&client, &symbol.to_uppercase());
    }

    if let Some(symbol) = matches.get_one::<String>("mid_point") {
        stream_mid_point(&client, &symbol.to_uppercase());
    }

    thread::sleep(Duration::from_secs(5));

    Ok(())
}

fn stream_last(client: &IBClient, symbol: &str) {
    // let bars = realtime::realtime_bars(&mut client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;

    // for (i, bar) in bars.enumerate() {
    //     println!("bar: {i:?} {bar:?}");

    //     if i > 60 {
    //         break;
    //     }
    // }

}

fn stream_all_last(client: &IBClient, symbol: &str) {
    
}

fn stream_bid_ask(client: &IBClient, symbol: &str) {
    
}

fn stream_mid_point(client: &IBClient, symbol: &str) {
    
}
