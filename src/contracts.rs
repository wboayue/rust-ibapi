use std::fmt::Debug;

use anyhow::{anyhow, Result};
use log::info;
use regex::Regex;

use crate::client::Client;
use crate::client::{RequestPacket, ResponsePacket};
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
    let mut message = promise.message()?;

    match message.message_type() {
        IncomingMessage::Error => Err(anyhow!("contract_details {:?}", message)),
        _ => {
            info!("inbound message: {:?}", message);
            decode_contract_details(client.server_version(), &mut message)
        }
    }
}

fn decode_contract_details(
    server_version: i32,
    message: &mut ResponsePacket,
) -> Result<ContractDetails> {
    message.skip(); // message type

    let mut message_version = 8;
    if server_version < server_versions::SIZE_RULES {
        message_version = message.next_int()?;
    }

    info!("server version: {} {}", server_version, message_version);

    let mut request_id = -1;
    if message_version >= 3 {
        request_id = message.next_int()?;
    }

    let mut contract = ContractDetails::default();

    contract.contract.symbol = message.next_string()?;
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, message, false);
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = message.next_string()?;
    contract.contract.currency = message.next_string()?;
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if server_version >= server_versions::MD_SIZE_MULTIPLIER && server_version < server_versions::SIZE_RULES {
        message.next_int();     // mdSizeMultiplier no longer used 
    }
    contract.contract.multiplier = message.next_string()?;
    contract.order_types = message.next_string()?;
    contract.valid_exchanges = message.next_string()?;
    if message_version >= 2 {
        contract.price_magnifier = message.next_int()?;
    }
    if message_version >= 4 {
        contract.under_contract_id = message.next_int()?;
    }
    if message_version >= 5 {
        //        https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EDecoder.cs#L1626
        contract.long_name = message.next_string()?;
        contract.contract.primary_exchange = message.next_string()?;
    }
    if message_version >= 6 {
        contract.contract_month = message.next_string()?;
        contract.industry = message.next_string()?;
        contract.category = message.next_string()?;
        contract.subcategory = message.next_string()?;
        contract.time_zone_id = message.next_string()?;
        contract.trading_hours = message.next_string()?;
        contract.liquid_hours = message.next_string()?;
    }
    if message_version >= 8 {
        contract.ev_rule = message.next_string()?;
        // contract.ev_multiplier = message.next_int()?; // FIXME int or double
    }

    Ok(contract)
}

fn read_last_trade_date(
    contract: &mut ContractDetails,
    message: &mut ResponsePacket,
    is_bond: bool,
) -> Result<()> {
    let mut last_trade_date_or_contract_month = message.next_string()?;
    if last_trade_date_or_contract_month.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = if last_trade_date_or_contract_month.contains("-") {
        last_trade_date_or_contract_month.split("-").collect()
    } else {
        // let re = Regex::new(r"\s+").unwrap();
        last_trade_date_or_contract_month.split(" ").collect()
    };

    if splitted.len() > 0 {
        if is_bond {
            contract.maturity = splitted[0].to_string();
        } else {
            contract.contract.last_trade_date_or_contract_month = splitted[0].to_string();
        }
    }
    if splitted.len() > 1 {
        contract.last_trade_time = splitted[1].to_string();
    }
    if is_bond && splitted.len() > 2 {
        contract.time_zone_id = splitted[2].to_string();
    }

    Ok(())
}
