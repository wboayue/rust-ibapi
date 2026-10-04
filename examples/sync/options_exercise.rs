//! Options Exercise example
//!
//! Finds a SPY call through the option chain and asks TWS to exercise one contract. Without a
//! position in that option, TWS answers with error 322.
//!
//! # Usage
//!
//! ```bash
//! cargo run --no-default-features --features sync --example options_exercise
//! ```

use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, OptionRight, SecurityType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;

    // SPY's contract id is 756733; the SMART chain lists every expiration and strike.
    let mut smart_chain = None;
    for chain in client.option_chain("SPY", SecurityType::Stock, 756733).subscribe()?.iter_data() {
        let chain = chain?;
        if chain.exchange == "SMART" {
            smart_chain = Some(chain);
            break;
        }
    }
    let chain = smart_chain.ok_or("no SMART option chain for SPY")?;

    let mut expirations = chain.expirations.clone();
    expirations.sort();
    let expiration = expirations.first().ok_or("no expirations")?;

    // The chain's strikes span every expiration. A strike of 0 asks TWS for each strike this
    // expiration lists; take the middle one.
    let mut calls = client.contract_details(&Contract::option("SPY", expiration, 0.0, OptionRight::Call))?;
    calls.sort_by(|a, b| a.contract.strike.total_cmp(&b.contract.strike));
    let contract = calls.get(calls.len() / 2).ok_or("no listed strikes")?.contract.clone();
    println!("Exercising 1 {} (contract id {})", contract.local_symbol, contract.contract_id);

    let accounts = client.managed_accounts()?;

    let subscription = client
        .exercise_options(&contract)
        .exercise(1)
        .account(&accounts[0])
        .override_natural_action() // exercise even if out of the money
        .submit()?;

    for event in subscription.iter_data() {
        match event {
            Ok(event) => println!("Response: {event:?}"),
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }

    Ok(())
}
