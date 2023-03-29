use super::Contract;
use crate::messages::OutgoingMessages;
use crate::messages::RequestMessage;
use crate::{server_versions, Error};

pub(crate) fn request_contract_data(server_version: i32, request_id: i32, contract: &Contract) -> Result<RequestMessage, Error> {
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

pub(crate) fn request_matching_symbols(request_id: i32, pattern: &str) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMatchingSymbols);
    message.push_field(&request_id);
    message.push_field(&pattern);

    Ok(message)
}

pub(crate) fn request_market_rule(market_rule_id: i32) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::default();

    message.push_field(&OutgoingMessages::RequestMarketRule);
    message.push_field(&market_rule_id);

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_market_rule() {
        let results = super::request_market_rule(26);

        match results {
            Ok(message) => {
                assert_eq!(message.encode(), "91\026\0", "message.encode()");
            }
            Err(err) => {
                assert!(false, "error encoding market rule request: {err}");
            }
        }
    }
}
