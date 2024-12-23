use std::thread;
use std::time::Duration;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::TickLast;
use ibapi::Client;

// This example demonstrates how to stream tick by tick data for the last price of a contract.

fn main() {
    env_logger::init();

    let connection_string = "127.0.0.1:4002";
    println!("connecting to server @ {connection_string}");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    // let contract = contract_es();
    let contract = Contract::stock("NVDA");
    let ticks = client.tick_by_tick_last(&contract, 0, false).expect("failed to get ticks");

    println!("streaming last price for security_type: {:?}, symbol: {}", contract.security_type, contract.symbol);

    for (i, tick) in ticks.timeout_iter(Duration::from_secs(10)).enumerate() {
        match tick {
            TickLast::Trade(trade) => {
                println!("{}: {i:?} {trade:?}", contract.symbol);
            }
            TickLast::Notice(notice) => {
                println!("error_code: {}, error_message: {}", notice.code, notice.message);
            }
        }
    }

    // check for errors during streaming
    if ticks.error().is_some() {
        println!("error: {}", ticks.error().unwrap());
    }    
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
