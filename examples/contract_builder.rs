use ibapi::contracts::{ComboLeg, Contract, ContractBuilder, SecurityType};

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

    // Example 4: Futures contract (US)
    println!("\n4. Futures Contract (US)");
    let futures_contract = ContractBuilder::futures("ES", "CME", "USD")
        .last_trade_date_or_contract_month("202503")
        .multiplier("50")
        .build()?;
    print_contract(&futures_contract);

    // Example 5: European futures contract
    println!("\n5. European Futures Contract");
    let european_futures = ContractBuilder::futures("GBL", "EUREX", "EUR")
        .last_trade_date_or_contract_month("202303")
        .build()?;
    print_contract(&european_futures);

    // Example 6: Futures contract with local symbol
    println!("\n6. Futures Contract with Local Symbol");
    let futures_local = ContractBuilder::new()
        .security_type(SecurityType::Future)
        .exchange("EUREX")
        .currency("EUR")
        .local_symbol("FGBL MAR 23")
        .last_trade_date_or_contract_month("202303")
        .build()?;
    print_contract(&futures_local);

    // Example 7: Crypto contract
    println!("\n7. Crypto Contract");
    let crypto_contract = ContractBuilder::crypto("BTC", "PAXOS", "USD").build()?;
    print_contract(&crypto_contract);

    // Example 8: Contract with specific ID
    println!("\n8. Contract by ID");
    let contract_by_id = ContractBuilder::new()
        .contract_id(265598) // AAPL contract ID (example)
        .exchange("SMART")
        .build()?;
    print_contract(&contract_by_id);

    // Example 9: Combo/Spread contract with legs
    println!("\n9. Combo/Spread Contract (Future Spread)");
    let leg_1 = ComboLeg {
        contract_id: 55928698, // WTI future June 2017
        ratio: 1,
        action: "BUY".to_string(),
        exchange: "IPE".to_string(),
        ..ComboLeg::default()
    };

    let leg_2 = ComboLeg {
        contract_id: 55850663, // COIL future June 2017
        ratio: 1,
        action: "SELL".to_string(),
        exchange: "IPE".to_string(),
        ..ComboLeg::default()
    };

    let combo_contract = ContractBuilder::new()
        .symbol("WTI") // WTI,COIL spread. Symbol can be defined as first leg symbol ("WTI") or currency ("USD")
        .security_type(SecurityType::Spread)
        .currency("USD")
        .exchange("SMART")
        .combo_legs(vec![leg_1, leg_2])
        .build()?;
    print_contract(&combo_contract);

    // Example 10: Complex option with all fields
    println!("\n10. Complex Option Contract");
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

    // Example 11: Error handling - missing required fields
    println!("\n11. Error Handling Examples");

    // Missing symbol, local_symbol, and contract_id
    match ContractBuilder::new().build() {
        Err(e) => println!("   ✓ Expected error for missing identifier: {e}"),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Option missing strike
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .right("C")
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for missing strike: {e}"),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Option missing right
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for missing right: {e}"),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Invalid option right
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("X") // Invalid - should be P or C
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for invalid right: {e}"),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Negative strike price
    match ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(-150.0)
        .right("C")
        .last_trade_date_or_contract_month("20250117")
        .build()
    {
        Err(e) => println!("   ✓ Expected error for negative strike: {e}"),
        Ok(_) => println!("   ✗ Unexpected success"),
    }

    // Futures missing contract month
    match ContractBuilder::futures("ES", "CME", "USD").build() {
        Err(e) => println!("   ✓ Expected error for missing contract month: {e}"),
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
    if !contract.symbol.is_empty() {
        println!("     Symbol: {}", contract.symbol);
    }
    if !contract.local_symbol.is_empty() {
        println!("     Local Symbol: {}", contract.local_symbol);
    }
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

    if !contract.combo_legs.is_empty() {
        println!("     Combo Legs: {} legs", contract.combo_legs.len());
        for (i, leg) in contract.combo_legs.iter().enumerate() {
            println!(
                "       Leg {}: Contract ID {}, Ratio {}, Action {}, Exchange {}",
                i + 1,
                leg.contract_id,
                leg.ratio,
                leg.action,
                leg.exchange
            );
        }
    }
}
