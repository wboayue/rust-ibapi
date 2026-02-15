#![allow(clippy::uninlined_format_args)]
use ibapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // SPY option — should have a last_trade_date
    let contract = Contract {
        symbol: Symbol::from("SPY"),
        security_type: SecurityType::Option,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "20260320".to_string(),
        strike: 600.0,
        right: "C".to_string(),
        ..Default::default()
    };

    let details = client.contract_details(&contract).await?;

    for (i, d) in details.iter().enumerate() {
        println!("--- Contract {} ---", i + 1);
        println!("  symbol:                            {}", d.contract.symbol);
        println!("  last_trade_date_or_contract_month: {}", d.contract.last_trade_date_or_contract_month);
        println!("  last_trade_date:                   {:?}", d.contract.last_trade_date);
        println!("  strike:                            {}", d.contract.strike);
        println!("  right:                             {}", d.contract.right);
        println!("  local_symbol:                      {}", d.contract.local_symbol);
    }

    // ES future — should also have a last_trade_date
    let contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "202606".to_string(),
        ..Default::default()
    };

    let details = client.contract_details(&contract).await?;

    for (i, d) in details.iter().enumerate() {
        println!("\n--- Future {} ---", i + 1);
        println!("  symbol:                            {}", d.contract.symbol);
        println!("  last_trade_date_or_contract_month: {}", d.contract.last_trade_date_or_contract_month);
        println!("  last_trade_date:                   {:?}", d.contract.last_trade_date);
        println!("  local_symbol:                      {}", d.contract.local_symbol);
    }

    Ok(())
}
