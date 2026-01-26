//! Example demonstrating the V2 contract builder API
//!
//! This example shows how to use the new type-safe contract builders
//! to create various types of contracts.

use ibapi::contracts::{Contract, ContractMonth, ExpirationDate, LegAction};

fn main() {
    // Simple stock contract
    let aapl = Contract::stock("AAPL").build();
    println!("Stock: {:?}", aapl.symbol);

    // Stock with customization
    let toyota = Contract::stock("7203").on_exchange("TSEJ").in_currency("JPY").primary("TSEJ").build();
    println!("International stock: {:?} on {}", toyota.symbol, toyota.exchange);

    // Call option - type-safe with required fields
    let call = Contract::call("AAPL").strike(150.0).expires_on(2024, 12, 20).build();
    println!(
        "Call option: {} {} strike {} exp {}",
        call.symbol, call.right, call.strike, call.last_trade_date_or_contract_month
    );

    // Put option with custom exchange
    let put = Contract::put("SPY")
        .strike(450.0)
        .expires(ExpirationDate::new(2024, 3, 15))
        .on_exchange("CBOE")
        .build();
    println!("Put option: {} {} strike {}", put.symbol, put.right, put.strike);

    // Futures contract with auto-calculated multiplier
    let es_futures = Contract::futures("ES").expires_in(ContractMonth::new(2024, 3)).build();
    println!("Futures: {} multiplier '{}'", es_futures.symbol, es_futures.multiplier);

    // Futures with specific month
    let cl_futures = Contract::futures("CL").expires_in(ContractMonth::new(2024, 6)).multiplier(1000).build();
    println!(
        "Futures: {} multiplier {} expiry {}",
        cl_futures.symbol, cl_futures.multiplier, cl_futures.last_trade_date_or_contract_month
    );

    // Forex pair
    let eur_usd = Contract::forex("EUR", "USD").build();
    println!("Forex: {}", eur_usd.symbol);

    // Cryptocurrency
    let btc = Contract::crypto("BTC").on_exchange("PAXOS").build();
    println!("Crypto: {} on {}", btc.symbol, btc.exchange);

    // Index contract
    let spx = Contract::index("SPX");
    println!("Index: {} on {}", spx.symbol, spx.exchange);

    // Spread contract
    let spread = Contract::spread()
        .calendar(12345, 67890) // Near and far contract IDs
        .build()
        .expect("Valid spread");
    println!("Calendar spread with {} legs", spread.combo_legs.len());

    // Custom spread with individual legs
    let custom_spread = Contract::spread()
        .add_leg(11111, LegAction::Buy)
        .ratio(2)
        .done()
        .add_leg(22222, LegAction::Sell)
        .ratio(1)
        .done()
        .build()
        .expect("Valid spread");
    println!("Custom spread with {} legs", custom_spread.combo_legs.len());

    // Demonstrating type safety - these would fail at compile time:
    // let invalid_call = Contract::call("AAPL").build(); // Won't compile - missing strike and expiry
    // let invalid_futures = Contract::futures("ES").build(); // Won't compile - missing expiry

    println!("\nAll contracts created successfully using V2 API!");
}
