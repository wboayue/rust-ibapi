use std::thread;
use std::time::Duration;

use clap::{arg, Command};

use ibapi::contracts::Contract;
use ibapi::Client;

fn main() {
    let matches = Command::new("tick_by_tick")
        .version("1.0")
        .author("Wil Boayue <wil@wsbsolutions.com")
        .about("Streams tick by tick data")
        .arg(arg!(--connection_string <VALUE>).default_value("localhost:4002"))
        .arg(arg!(--last <SYMBOL>))
        .arg(arg!(--all_last <SYMBOL>))
        .arg(arg!(--bid_ask <SYMBOL>))
        .arg(arg!(--mid_point <SYMBOL>))
        .get_matches();

    let connection_string = matches.get_one::<String>("connection_string").expect("connection_string is required");
    println!("connection_string: {connection_string}");

    let mut client = Client::connect(connection_string, 100).expect("connection failed");

    if let Some(symbol) = matches.get_one::<String>("last") {
        stream_last(&mut client, &symbol.to_uppercase()).unwrap();
    }

    if let Some(symbol) = matches.get_one::<String>("all_last") {
        stream_all_last(&mut client, &symbol.to_uppercase()).unwrap();
    }

    if let Some(symbol) = matches.get_one::<String>("bid_ask") {
        stream_bid_ask(&mut client, &symbol.to_uppercase()).unwrap();
    }

    if let Some(symbol) = matches.get_one::<String>("mid_point") {
        stream_mid_point(&mut client, &symbol.to_uppercase()).unwrap();
    }

    thread::sleep(Duration::from_secs(5));
}

fn stream_last(client: &mut Client, _symbol: &str) -> anyhow::Result<()> {
    let contract = contract_gc();
    let ticks = client.tick_by_tick_last(&contract, 0, false)?;

    for (i, tick) in ticks.enumerate() {
        println!("{}: {i:?} {tick:?}", contract.symbol);
    }

    Ok(())
}

fn contract_es() -> Contract {
    let mut contract = Contract::futures("ES");
    contract.local_symbol = "ESU3".to_string();
    contract.trading_class = "ES".into();
    contract.exchange = "CME".into();
    contract
}

fn contract_gc() -> Contract {
    let mut contract = Contract::futures("GC");
    contract.exchange = "COMEX".to_owned();
    contract.local_symbol = "GCZ3".to_string();
    contract.trading_class = "GC".into();
    contract
}

fn contract_zn() -> Contract {
    let mut contract = Contract::futures("ZN");
    contract.exchange = "CBOT".to_owned();
    contract.contract_id = 568735904;
    contract
}

fn stream_all_last(client: &Client, symbol: &str) -> anyhow::Result<()> {
    let contract = contract_es();
    let ticks = client.tick_by_tick_all_last(&contract, 0, false)?;

    for (i, tick) in ticks.enumerate().take(60) {
        println!("tick: {i:?} {tick:?}");
    }

    Ok(())
}

fn stream_bid_ask(client: &mut Client, _symbol: &str) -> anyhow::Result<()> {
    let contract = contract_es();
    let ticks = client.tick_by_tick_bid_ask(&contract, 0, false)?;

    for (i, tick) in ticks.enumerate() {
        println!("tick: {i:?} {tick:?}");
    }

    Ok(())
}

fn stream_mid_point(client: &mut Client, _symbol: &str) -> anyhow::Result<()> {
    let contract = contract_es();
    let ticks = client.tick_by_tick_midpoint(&contract, 0, false)?;

    for (i, tick) in ticks.iter().enumerate().take(60) {
        println!("tick: {i:?} {tick:?}");
    }

    Ok(())
}
