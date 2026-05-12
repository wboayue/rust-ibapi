//! Options Exercise example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example options_exercise
//! ```

use ibapi::client::blocking::Client;
use ibapi::{
    contracts::{Contract, SecurityType},
    orders::ExerciseAction,
};

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    // Try to get option chain first
    println!("Attempting to get SPY option chain...");
    let option_chain_result = client.option_chain("SPY", "", SecurityType::Stock, 0);

    let mut option_contract = if let Ok(subscription) = option_chain_result {
        let mut chains = Vec::new();
        for chain in subscription.iter_data() {
            let chain = match chain {
                Ok(chain) => chain,
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
            };
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
            let (year, month, day) = parse_yyyymmdd(expiration).expect("expiration in YYYYMMDD");

            println!("Using option from chain: SPY {} Call, Strike: {}", expiration, strike);

            Contract::call("SPY")
                .strike(strike)
                .expires_on(year, month, day)
                .on_exchange(chain.exchange.clone())
                .multiplier(chain.multiplier.parse().unwrap_or(100))
                .trading_class(chain.trading_class.clone())
                .build()
        } else {
            println!("No option chain data available, using hardcoded contract");
            Contract::call("SPY").strike(500.0).expires_on(2025, 1, 17).build()
        }
    } else {
        println!("Could not get option chain, using hardcoded contract");
        Contract::call("SPY").strike(550.0).expires_on(2025, 1, 17).build()
    };

    println!("\nGetting contract details for option:");
    println!(
        "Symbol: {}, Strike: {}, Right: {}, Expiry: {}",
        option_contract.symbol,
        option_contract.strike,
        option_contract.right.map_or("", |r| r.as_str()),
        option_contract.last_trade_date_or_contract_month
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

    for status in subscription.iter_data() {
        match status {
            Ok(status) => println!("Response: {:?}", status),
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }

    println!("\nExercise options example completed.");
}

fn parse_yyyymmdd(s: &str) -> Option<(u16, u8, u8)> {
    if s.len() != 8 {
        return None;
    }
    let year = s.get(..4)?.parse().ok()?;
    let month = s.get(4..6)?.parse().ok()?;
    let day = s.get(6..8)?.parse().ok()?;
    Some((year, month, day))
}
