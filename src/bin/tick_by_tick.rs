use std::thread;
use std::time::Duration;

use clap::{arg, Command};

use ibapi::client::IBClient;
use ibapi::contracts::Contract;
use ibapi::market_data::realtime;

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

    let mut client = IBClient::connect(&connection_string)?;

    if let Some(symbol) = matches.get_one::<String>("last") {
        stream_last(&mut client, &symbol.to_uppercase())?;
    }

    if let Some(symbol) = matches.get_one::<String>("all_last") {
        stream_all_last(&mut client, &symbol.to_uppercase())?;
    }

    if let Some(symbol) = matches.get_one::<String>("bid_ask") {
        stream_bid_ask(&mut client, &symbol.to_uppercase());
    }

    if let Some(symbol) = matches.get_one::<String>("mid_point") {
        stream_mid_point(&mut client, &symbol.to_uppercase());
    }

    thread::sleep(Duration::from_secs(5));

    Ok(())
}

fn stream_last(client: &mut IBClient, symbol: &str) -> anyhow::Result<()> {
    let contract = Contract::stock(symbol);
    let ticks = realtime::tick_by_tick_last(client, &contract, 0, false)?;

    for (i, tick) in ticks.enumerate().take(60) {
        println!("tick: {i:?} {tick:?}");
    }

    Ok(())
}

fn stream_all_last(client: &mut IBClient, symbol: &str) -> anyhow::Result<()> {
    let contract = Contract::stock(symbol);
    let ticks = realtime::tick_by_tick_all_last(client, &contract, 0, false)?;

    for (i, tick) in ticks.enumerate().take(60) {
        println!("tick: {i:?} {tick:?}");
    }

    Ok(())
}

fn stream_bid_ask(client: &mut IBClient, symbol: &str) {
    let contract = Contract::stock(symbol);
    // let ticks = realtime::tick_by_tick_bid_ask(client, &contract, 0, false)?;

    // for (i, tick) in ticks.enumerate().take(60) {
    //     println!("tick: {i:?} {tick:?}");
    // }

    // Ok(())
}

fn stream_mid_point(client: &mut IBClient, symbol: &str) {
    let contract = Contract::stock(symbol);
    // let ticks = realtime::tick_by_tick_midpoint(client, &contract, 0, false)?;

    // for (i, tick) in ticks.enumerate().take(60) {
    //     println!("tick: {i:?} {tick:?}");
    // }

    // Ok(())
}
