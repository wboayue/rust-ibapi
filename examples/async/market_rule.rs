use ibapi::contracts::{Contract, SecurityType};
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // First, get contract details to find market rule IDs
    let contract = Contract {
        symbol: "ES".to_string(),
        security_type: SecurityType::Future,
        exchange: "CME".to_string(),
        currency: "USD".to_string(),
        last_trade_date_or_contract_month: "202503".to_string(),
        ..Default::default()
    };

    println!("Getting contract details for ES futures...");
    let contract_details = client.contract_details(&contract).await?;

    if let Some(details) = contract_details.first() {
        println!("Contract: {} on {}", details.contract.local_symbol, details.contract.exchange);

        if !details.market_rule_ids.is_empty() {
            println!("Market Rule IDs: {}", details.market_rule_ids.join(", "));

            // Parse the first market rule ID
            if let Some(rule_id_str) = details.market_rule_ids.first() {
                if let Ok(market_rule_id) = rule_id_str.trim().parse::<i32>() {
                    println!("\n=== Fetching Market Rule {} ===", market_rule_id);

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
                        Err(e) => eprintln!("Error fetching market rule: {:?}", e),
                    }
                }
            }
        } else {
            println!("No market rule IDs found for this contract");
        }
    } else {
        println!("No contract details found");
    }

    Ok(())
}
