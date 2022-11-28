use std::fmt::Debug;

use anyhow::{anyhow, Result};
use log::info;

use crate::client::Client;
use crate::client::RequestPacket;
use crate::domain::Contract;
use crate::domain::ContractDetails;
use crate::domain::DeltaNeutralContract;
use crate::domain::SecurityType;
use crate::messages::{IncomingMessage, OutgoingMessage};
use crate::server_versions;

pub fn stock(symbol: &str) -> Contract {
    Contract {
        symbol: symbol.to_string(),
        ..default()
    }
}

pub fn default() -> Contract {
    Contract {
        contract_id: 0,
        symbol: "".to_string(),
        security_type: SecurityType::STK,
        last_trade_date_or_contract_month: "".to_string(),
        strike: 0.0,
        right: "".to_string(),
        multiplier: "".to_string(),
        exchange: "".to_string(),
        currency: "".to_string(),
        local_symbol: "".to_string(),
        primary_exchange: "".to_string(),
        trading_class: "".to_string(),
        include_expired: false,
        security_id_type: "".to_string(),
        security_id: "".to_string(),
        combo_legs_description: "".to_string(),
        combo_legs: Vec::new(),
        issuer_id: "".to_string(),
        delta_neutral_contract: DeltaNeutralContract {
            contract_id: "".to_string(),
            delta: 1.0,
            price: 12.0,
        },
    }
}

/// Requests contract information.
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. This information will be returned at EWrapper:contractDetails. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
pub fn contract_details<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
) -> Result<ContractDetails> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(
            server_versions::SEC_ID_TYPE,
            "It does not support security_id_type or security_id attributes",
        )?
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support the trading_class parameter when requesting contract details.",
        )?
    }

    if !contract.primary_exchange.is_empty() {
        client.check_server_version(
            server_versions::LINKING,
            "It does not support primary_exchange parameter when requesting contract details.",
        )?
    }

    if !contract.issuer_id.is_empty() {
        client.check_server_version(
            server_versions::BOND_ISSUERID,
            "It does not support issuer_id parameter when requesting contract details.",
        )?
    }

    const VERSION: i32 = 8;

    let request_id = client.next_request_id();
    let mut packet = RequestPacket::default();

    packet.add_field(&OutgoingMessage::RequestContractData);
    packet.add_field(&VERSION);

    if client.server_version() >= server_versions::CONTRACT_DATA_CHAIN {
        packet.add_field(&request_id);
    }

    if client.server_version() >= server_versions::CONTRACT_CONID {
        packet.add_field(&contract.contract_id);
    }

    packet.add_field(&contract.symbol);
    packet.add_field(&contract.security_type);
    packet.add_field(&contract.last_trade_date_or_contract_month);
    packet.add_field(&contract.strike);
    packet.add_field(&contract.right);

    if client.server_version() >= 15 {
        packet.add_field(&contract.multiplier);
    }

    if client.server_version() >= server_versions::PRIMARYEXCH {
        packet.add_field(&contract.exchange);
        packet.add_field(&contract.primary_exchange);
    } else if client.server_version() >= server_versions::LINKING {
        if !contract.primary_exchange.is_empty()
            && (contract.exchange == "BEST" || contract.exchange == "SMART")
        {
            packet.add_field(&format!(
                "{}:{}",
                contract.exchange, contract.primary_exchange
            ));
        } else {
            packet.add_field(&contract.exchange);
        }
    }

    packet.add_field(&contract.currency);
    packet.add_field(&contract.local_symbol);

    if client.server_version() >= server_versions::TRADING_CLASS {
        packet.add_field(&contract.trading_class);
    }
    if client.server_version() >= 31 {
        packet.add_field(&contract.include_expired);
    }
    if client.server_version() >= server_versions::SEC_ID_TYPE {
        packet.add_field(&contract.security_id_type);
        packet.add_field(&contract.security_id);
    }
    if client.server_version() >= server_versions::BOND_ISSUERID {
        packet.add_field(&contract.issuer_id);
    }

    info!("outbound message: {:?}", packet);

    let promise = client.send_message(request_id, packet)?;
    let message = promise.message()?;

    match message.message_type() {
        IncomingMessage::Error => Err(anyhow!("contract_details {:?}", message)),
        _ => { 
            info!("inbound message: {:?}", message);
            Ok(ContractDetails::default())
        }
    }
}

// client.reqMatchingSymbols(211, "IBM");
