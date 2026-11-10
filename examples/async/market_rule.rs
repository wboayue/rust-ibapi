#![allow(clippy::uninlined_format_args)]
use ibapi::contracts::{Contract, ContractMonth};
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // First, get contract details to find market rule IDs
    let contract = Contract::futures("ES").expires_in(ContractMonth::new(2025, 3)).build();

    println!("Getting contract details for ES futures...");
    let contract_details = client.contract_details(&contract).await?;

    if let Some(details) = contract_details.first() {
        println!("Contract: {} on {}", details.contract.local_symbol, details.contract.exchange);

        if let Some(market_rule_id) = details.market_rule_id(details.contract.exchange.as_str()) {
            println!("Market Rule IDs: {:?} for {:?}", details.market_rule_ids, details.valid_exchanges);
            println!("\n=== Fetching Market Rule {market_rule_id} ===");

            match client.market_rule(market_rule_id).await {
                Ok(rule) => {
                    println!("Market Rule ID: {}", rule.market_rule_id);
                    println!("Price Increments:");

                    for increment in &rule.price_increments {
                        if increment.low_edge < f64::MAX {
                            println!("  From {:.2}: increment = {}", increment.low_edge, increment.increment);
                        } else {
                            println!("  Default: increment = {}", increment.increment);
                        }
                    }
                }
                Err(e) => eprintln!("Error fetching market rule: {e:?}"),
            }
        } else {
            println!("No market rule ID found for {}", details.contract.exchange);
        }
    } else {
        println!("No contract details found");
    }

    Ok(())
}
