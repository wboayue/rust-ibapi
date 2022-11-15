use std::fmt::Debug;

use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::domain::Contract;
use crate::domain::ContractDetails;
use crate::domain::DeltaNeutralContract;
use crate::domain::SecurityType;

pub fn stock(symbol: &str) -> Contract {
    Contract {
        symbol: symbol.to_string(),
        ..default()
    }
}

pub fn default() -> Contract {
    Contract {
        contract_id: 1,
        symbol: "".to_string(),
        security_type: SecurityType::STK,
        last_trade_date_or_contract_month: "123".to_string(),
        strike: 0.0,
        right: "".to_string(),
        multiplier: "".to_string(),
        exchange: "".to_string(),
        currency: "".to_string(),
        local_symbol: "".to_string(),
        primary_exchange: "".to_string(),
        trading_class: "".to_string(),
        include_expired: true,
        security_id_type: "".to_string(),
        security_id: "".to_string(),
        combo_legs_description: "".to_string(),
        combo_legs: Vec::new(),
        delta_neutral_contract: DeltaNeutralContract {
            contract_id: "".to_string(),
            delta: 1.0,
            price: 12.0,
        },
    }
}

/// Requests contract information.
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. This information will be returned at EWrapper:contractDetails. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
pub fn contract_details<C: Client+Debug>(client: &C, contract: &Contract) -> Result<ContractDetails> {
    print!("{:?} {:?}", client, contract);
    Err(anyhow!("not implemented!"))
}
