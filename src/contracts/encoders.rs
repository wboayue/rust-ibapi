use super::Contract;
use super::SecurityType;
use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::{server_versions, Error};

#[cfg(test)]
mod tests;

pub(crate) fn encode_request_contract_data(server_version: i32, request_id: i32, contract: &Contract) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 8;

    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestContractData);
    packet.push_field(&VERSION);

    if server_version >= server_versions::CONTRACT_DATA_CHAIN {
        packet.push_field(&request_id);
    }

    if server_version >= server_versions::CONTRACT_CONID {
        packet.push_field(&contract.contract_id);
    }

    packet.push_field(&contract.symbol);
    packet.push_field(&contract.security_type);
    packet.push_field(&contract.last_trade_date_or_contract_month);
    packet.push_field(&contract.strike);
    packet.push_field(&contract.right);

    if server_version >= 15 {
        packet.push_field(&contract.multiplier);
    }

    if server_version >= server_versions::PRIMARYEXCH {
        packet.push_field(&contract.exchange);
        packet.push_field(&contract.primary_exchange);
    } else if server_version >= server_versions::LINKING {
        if !contract.primary_exchange.is_empty() && (contract.exchange == "BEST" || contract.exchange == "SMART") {
            packet.push_field(&format!("{}:{}", contract.exchange, contract.primary_exchange));
        } else {
            packet.push_field(&contract.exchange);
        }
    }

    packet.push_field(&contract.currency);
    packet.push_field(&contract.local_symbol);

    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.trading_class);
    }
    if server_version >= 31 {
        packet.push_field(&contract.include_expired);
    }
    if server_version >= server_versions::SEC_ID_TYPE {
        packet.push_field(&contract.security_id_type);
        packet.push_field(&contract.security_id);
    }
    if server_version >= server_versions::BOND_ISSUERID {
        packet.push_field(&contract.issuer_id);
    }

    Ok(packet)
}

pub(crate) fn encode_request_matching_symbols(request_id: i32, pattern: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMatchingSymbols);
    message.push_field(&request_id);
    message.push_field(&pattern);

    Ok(message)
}

pub(crate) fn encode_request_market_rule(market_rule_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMarketRule);
    message.push_field(&market_rule_id);

    Ok(message)
}

pub(crate) fn encode_calculate_option_price(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    volatility: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    encode_option_computation(server_version, request_id, contract, volatility, underlying_price)
}

pub(crate) fn encode_calculate_implied_volatility(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    encode_option_computation(server_version, request_id, contract, option_price, underlying_price)
}

pub(crate) fn encode_option_computation(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    price_or_volatility: f64,
    underlying_price: f64,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 3;

    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::ReqCalcImpliedVolat);
    message.push_field(&VERSION);
    message.push_field(&request_id);
    encode_contract(server_version, &mut message, contract);
    message.push_field(&price_or_volatility);
    message.push_field(&underlying_price);
    if server_version >= server_versions::LINKING {
        message.push_field(&"");
    }

    Ok(message)
}

fn encode_contract(server_version: i32, message: &mut RequestMessage, contract: &Contract) {
    message.push_field(&contract.contract_id);
    message.push_field(&contract.symbol);
    message.push_field(&contract.security_type);
    message.push_field(&contract.last_trade_date_or_contract_month);
    message.push_field(&contract.strike);
    message.push_field(&contract.right);
    message.push_field(&contract.multiplier);
    message.push_field(&contract.exchange);
    message.push_field(&contract.primary_exchange);
    message.push_field(&contract.currency);
    message.push_field(&contract.local_symbol);
    if server_version >= server_versions::TRADING_CLASS {
        message.push_field(&contract.trading_class);
    }
}

pub(crate) fn encode_cancel_option_computation(message_type: OutgoingMessages, request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::default();

    message.push_field(&message_type);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}

pub(super) fn encode_request_option_chain(
    request_id: i32,
    symbol: &str,
    exchange: &str,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestSecurityDefinitionOptionalParameters);
    message.push_field(&request_id);
    message.push_field(&symbol);
    message.push_field(&exchange);
    message.push_field(&security_type);
    message.push_field(&contract_id);

    Ok(message)
}
