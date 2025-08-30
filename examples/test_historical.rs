use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::Client;
use time::macros::datetime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable recording to capture messages
    std::env::set_var("IBAPI_RECORDING_DIR", "/tmp/tws-messages");
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    // Test head_timestamp
    let contract = Contract::stock("AAPL");

    println!("Testing head_timestamp...");
    let timestamp = client.head_timestamp(&contract, WhatToShow::Trades, true)?;
    println!("Head timestamp: {:?}", timestamp);

    // Test historical_data
    println!("\nTesting historical_data...");
    let end_date = datetime!(2024-01-15 16:00:00).assume_utc();
    let data = client.historical_data(&contract, Some(end_date), Duration::days(1), BarSize::Min5, WhatToShow::Trades, true)?;
    println!("Got {} bars", data.bars.len());
    if !data.bars.is_empty() {
        println!("First bar: {:?}", data.bars[0]);
    }

    // Test histogram_data
    println!("\nTesting histogram_data...");
    let histogram = client.histogram_data(&contract, true, BarSize::Day)?;
    println!("Got {} histogram entries", histogram.len());
    if !histogram.is_empty() {
        println!("First entry: {:?}", histogram[0]);
    }

    Ok(())
}
