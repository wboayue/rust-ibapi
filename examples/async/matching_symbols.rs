use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // Search for symbols matching a pattern
    let patterns = vec!["AAPL", "MICRO", "EUR"];

    for pattern in patterns {
        println!("\n=== Searching for symbols matching '{}' ===", pattern);
        
        let matches = client.matching_symbols(pattern).await?;
        
        if matches.is_empty() {
            println!("No matches found for '{}'", pattern);
        } else {
            println!("Found {} matches:", matches.len());
            
            for (i, contract_desc) in matches.iter().enumerate() {
                println!("\n  Match {}):", i + 1);
                println!("    Symbol: {}", contract_desc.contract.symbol);
                println!("    Security Type: {:?}", contract_desc.contract.security_type);
                println!("    Primary Exchange: {}", contract_desc.contract.primary_exchange);
                println!("    Currency: {}", contract_desc.contract.currency);
                
                if !contract_desc.derivative_security_types.is_empty() {
                    println!("    Derivative Types: {}", contract_desc.derivative_security_types.join(", "));
                }
                
                if !contract_desc.description.is_empty() {
                    println!("    Description: {}", contract_desc.description);
                }
            }
        }
    }

    Ok(())
}