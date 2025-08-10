//! Options Exercise example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example options_exercise
//! ```

use ibapi::{
    contracts::{Contract, SecurityType},
    orders::ExerciseAction,
    Client,
};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // Try to get option chain first
    println!("Attempting to get SPY option chain...");
    let option_chain_result = client.option_chain("SPY", "", SecurityType::Stock, 0);

    let mut option_contract = if let Ok(subscription) = option_chain_result {
        let mut chains = Vec::new();
        for chain in subscription {
            println!(
                "Found option chain for exchange: {}, trading class: {}",
                chain.exchange, chain.trading_class
            );
            chains.push(chain);
        }

        if !chains.is_empty() && !chains[0].expirations.is_empty() && !chains[0].strikes.is_empty() {
            let chain = &chains[0];
            let expiration = &chain.expirations[0];
            let strike = chain.strikes[chain.strikes.len() / 2];

            println!("Using option from chain: SPY {} Call, Strike: {}", expiration, strike);

            Contract {
                symbol: "SPY".to_owned(),
                security_type: SecurityType::Option,
                exchange: chain.exchange.clone(),
                currency: "USD".to_owned(),
                last_trade_date_or_contract_month: expiration.clone(),
                strike,
                right: "C".to_owned(),
                multiplier: chain.multiplier.clone(),
                trading_class: chain.trading_class.clone(),
                ..Default::default()
            }
        } else {
            println!("No option chain data available, using hardcoded contract");
            // Fallback to hardcoded contract
            Contract {
                symbol: "SPY".to_owned(),
                security_type: SecurityType::Option,
                exchange: "SMART".to_owned(),
                currency: "USD".to_owned(),
                last_trade_date_or_contract_month: "20250117".to_owned(),
                strike: 500.0, // More reasonable strike for SPY
                right: "C".to_owned(),
                multiplier: "100".to_owned(),
                ..Default::default()
            }
        }
    } else {
        println!("Could not get option chain, using hardcoded contract");
        // Fallback to hardcoded contract
        Contract {
            symbol: "SPY".to_owned(),
            security_type: SecurityType::Option,
            exchange: "SMART".to_owned(),
            currency: "USD".to_owned(),
            last_trade_date_or_contract_month: "20250117".to_owned(),
            strike: 550.0,
            right: "C".to_owned(),
            multiplier: "100".to_owned(),
            ..Default::default()
        }
    };

    println!("\nGetting contract details for option:");
    println!(
        "Symbol: {}, Strike: {}, Right: {}, Expiry: {}",
        option_contract.symbol, option_contract.strike, option_contract.right, option_contract.last_trade_date_or_contract_month
    );

    // Try to get contract details to validate and get the full contract info
    match client.contract_details(&option_contract) {
        Ok(details) if !details.is_empty() => {
            // Use the first valid contract with full details
            option_contract = details[0].contract.clone();
            println!("\nFound valid contract!");
            println!("Local Symbol: {}", option_contract.local_symbol);
            println!("Contract ID: {}", option_contract.contract_id);
            println!("Exchange: {}", option_contract.exchange);
            println!("Trading Class: {}", option_contract.trading_class);
        }
        _ => {
            println!("\nWarning: Could not validate contract details.");
            println!("Will attempt to exercise with the provided contract specification.");
            println!("Note: This may fail if the contract doesn't exist.");
        }
    }

    let accounts = client.managed_accounts().expect("could not get managed accounts");
    let account = &accounts[0];
    let manual_order_time = None;

    println!("\n=== Exercising Option Contract ===");
    println!("Account: {}", account);
    println!("Action: Exercise");
    println!("Quantity: 1 contract");
    println!("Override: true (exercise even if out-of-the-money)");
    println!();

    let subscription = client
        .exercise_options(&option_contract, ExerciseAction::Exercise, 1, account, true, manual_order_time)
        .expect("exercise options request failed!");

    println!("Exercise request sent. Waiting for responses...\n");

    for status in &subscription {
        println!("Response: {:?}", status);
    }

    println!("\nExercise options example completed.");
}
