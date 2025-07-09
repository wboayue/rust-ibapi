use ibapi::contracts::{Contract, ContractBuilder, SecurityType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Contract Builder Examples ===\n");

    // Example 1: Stock contract using builder pattern
    println!("1. Stock Contract (Builder Pattern)");
    let stock_contract = ContractBuilder::new()
        .symbol("MSFT")
        .security_type(SecurityType::Stock)
        .exchange("SMART")
        .currency("USD")
        .primary_exchange("NASDAQ")
        .build()?;
    print_contract(&stock_contract);

    // Example 2: Stock contract using convenience method
    println!("\n2. Stock Contract (Convenience Method)");
    let stock_contract = ContractBuilder::stock("AAPL", "SMART", "USD").primary_exchange("NASDAQ").build()?;
    print_contract(&stock_contract);

    // Example 3: Option contract
    println!("\n3. Option Contract");
    let option_contract = ContractBuilder::option("SPY", "SMART", "USD")
        .strike(450.0)
        .right("C")
        .last_trade_date_or_contract_month("20250117")
        .multiplier("100")
        .build()?;
    print_contract(&option_contract);

    // Example 4: Futures contract
    println!("\n4. Futures Contract");
    let futures_contract = ContractBuilder::futures("ES", "CME", "USD")
        .last_trade_date_or_contract_month("202503")
        .multiplier("50")
        .build()?;
    print_contract(&futures_contract);

    // Example 5: Crypto contract
    println!("\n5. Crypto Contract");
    let crypto_contract = ContractBuilder::crypto("BTC", "PAXOS", "USD").build()?;
    print_contract(&crypto_contract);

    // Example 6: Contract with specific ID
    println!("\n6. Contract by ID");
    let contract_by_id = ContractBuilder::new()
        .contract_id(265598) // AAPL contract ID (example)
        .exchange("SMART")
        .build()?;
    print_contract(&contract_by_id);

    // Example 7: Complex option with all fields
    println!("\n7. Complex Option Contract");
    let complex_option = ContractBuilder::new()
        .symbol("TSLA")
        .security_type(SecurityType::Option)
        .exchange("SMART")
        .currency("USD")
        .strike(200.0)
        .right("P") // Put option
        .last_trade_date_or_contract_month("20250320")
        .multiplier("100")
        .trading_class("TSLA")
        .include_expired(false)
        .primary_exchange("NASDAQOM")
        .build()?;
    print_contract(&complex_option);

    // Example 8: Error handling - missing required fields
    println!("\n8. Error Handling Examples");

    // Missing symbol
    match ContractBuilder::new().build() {
        Err(e) => println!("   ✓ Expected error for missing symbol: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Option missing strike
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .right("C")
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for missing strike: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Option missing right
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for missing right: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Invalid option right
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("X") // Invalid - should be P or C
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for invalid right: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Negative strike price
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(-150.0)
        .right("C")
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for negative strike: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Futures missing contract month
    match ContractBuilder::futures("ES", "CME", "USD").build() {
        Err(e) => println!("   ✓ Expected error for missing contract month: {}", e),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    println!("\n=== Contract Builder Examples Complete ===");
    Ok(())
}

fn print_contract(contract: &Contract) {
    println!("   Contract Details:");
    if contract.contract_id > 0 {
        println!("     Contract ID: {}", contract.contract_id);
    }
    println!("     Symbol: {}", contract.symbol);
    println!("     Security Type: {:?}", contract.security_type);
    println!("     Exchange: {}", contract.exchange);
    println!("     Currency: {}", contract.currency);

    if !contract.primary_exchange.is_empty() {
        println!("     Primary Exchange: {}", contract.primary_exchange);
    }

    if contract.security_type == SecurityType::Option || contract.security_type == SecurityType::FuturesOption {
        println!("     Strike: {}", contract.strike);
        println!("     Right: {}", contract.right);
        println!("     Expiry: {}", contract.last_trade_date_or_contract_month);
    }

    if contract.security_type == SecurityType::Future || contract.security_type == SecurityType::FuturesOption {
        println!("     Contract Month: {}", contract.last_trade_date_or_contract_month);
    }

    if !contract.multiplier.is_empty() {
        println!("     Multiplier: {}", contract.multiplier);
    }

    if !contract.trading_class.is_empty() {
        println!("     Trading Class: {}", contract.trading_class);
    }
}
