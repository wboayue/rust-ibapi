#![allow(clippy::uninlined_format_args)]
use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // Create a contract for Apple stock
    let contract = Contract {
        symbol: Symbol::from("AAPL"),
        security_type: SecurityType::Stock,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    };

    // Request contract details
    let contract_details = client.contract_details(&contract).await?;

    println!("Found {} contracts matching the criteria", contract_details.len());

    for (i, details) in contract_details.iter().enumerate() {
        println!("\n--- Contract {} ---", i + 1);
        println!("Symbol: {}", details.contract.symbol);
        println!("Local Symbol: {}", details.contract.local_symbol);
        println!("Contract ID: {}", details.contract.contract_id);
        println!("Exchange: {}", details.contract.exchange);
        println!("Primary Exchange: {}", details.contract.primary_exchange);
        println!("Currency: {}", details.contract.currency);
        println!("Long Name: {}", details.long_name);
        println!("Industry: {}", details.industry);
        println!("Category: {}", details.category);
        println!("Subcategory: {}", details.subcategory);

        if !details.valid_exchanges.is_empty() {
            println!("Valid Exchanges: {}", details.valid_exchanges.join(", "));
        }

        // min_tick is not an Option, it's an f64
        println!("Min Tick: {}", details.min_tick);
    }

    Ok(())
}
