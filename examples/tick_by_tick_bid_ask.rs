use std::time::Duration;

use ibapi::contracts::Contract;
use ibapi::market_data::realtime::BidAskTicks;
use ibapi::Client;

// This example demonstrates how to stream tick by tick data for the bid and ask price of a contract.

fn main() {
    env_logger::init();

    let connection_string = "127.0.0.1:4002";
    println!("connecting to server @ {connection_string}");

    let client = Client::connect(connection_string, 100).expect("connection failed");

    let contract = Contract::stock("NVDA");
    let ticks = client.tick_by_tick_bid_ask(&contract, 0, false).expect("failed to get ticks");

    println!(
        "streaming bid/ask price for security_type: {:?}, symbol: {}",
        contract.security_type, contract.symbol
    );

    for (i, tick) in ticks.timeout_iter(Duration::from_secs(10)).enumerate() {
        match tick {
            BidAskTicks::BidAsk(bid_ask) => {
                println!("{}: {i:?} {bid_ask:?}", contract.symbol);
            }
            BidAskTicks::Notice(notice) => {
                // server could send a notice if it doesn't recognize the contract
                println!("error_code: {}, error_message: {}", notice.code, notice.message);
            }
        }
    }

    // check for errors during streaming
    if let Some(error) = ticks.error() {
        println!("error: {}", error);
    }
}
