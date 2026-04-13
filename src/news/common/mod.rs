pub(crate) mod decoders;
pub(crate) mod encoders;
pub(crate) mod stream_decoders;

use crate::contracts::Contract;
use crate::Error;

/// Build the generic tick list for contract-specific news subscriptions.
pub(crate) fn contract_news_generic_ticks(provider_codes: &[&str]) -> Vec<String> {
    let mut ticks = vec!["mdoff".to_string()];
    for provider in provider_codes {
        ticks.push(format!("292:{provider}"));
    }
    ticks
}

/// Encode a contract news market data request.
pub(crate) fn encode_contract_news_request(request_id: i32, contract: &Contract, provider_codes: &[&str]) -> Result<Vec<u8>, Error> {
    let generic_ticks = contract_news_generic_ticks(provider_codes);
    let generic_ticks: Vec<_> = generic_ticks.iter().map(|s| s.as_str()).collect();
    crate::market_data::realtime::common::encoders::encode_request_market_data(request_id, contract, generic_ticks.as_slice(), false, false)
}

/// Encode a broad tape news market data request.
pub(crate) fn encode_broad_tape_news_request(request_id: i32, provider_code: &str) -> Result<Vec<u8>, Error> {
    let contract = Contract::news(provider_code);
    let generic_ticks = &["mdoff", "292"];
    crate::market_data::realtime::common::encoders::encode_request_market_data(request_id, &contract, generic_ticks, false, false)
}
