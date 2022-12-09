use std::fmt::Debug;

use anyhow::{anyhow, Result};
use log::{error, info};

use crate::client::{Client, RequestPacket, ResponsePacket};
use crate::domain::Contract;
use crate::domain::ContractDetails;
use crate::domain::DeltaNeutralContract;
use crate::domain::SecurityType;
use crate::domain::TagValue;
use crate::messages::{IncomingMessage, OutgoingMessage};
use crate::server_versions;

/// Creates stock contract from specified symbol
pub fn stock(symbol: &str) -> Contract {
    Contract {
        symbol: symbol.to_string(),
        security_type: SecurityType::STK,
        ..default()
    }
}

/// Creates a default contract
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
        description: "".to_string(),
        delta_neutral_contract: DeltaNeutralContract {
            contract_id: "".to_string(),
            delta: 0.0,
            price: 0.0,
        },
    }
}

/// Requests contract information.
///
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::BasicClient;
/// use ibapi::contracts;
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = BasicClient::connect("localhost:4002")?;
///
///     let contract = contracts::stock("TSLA");
///     let results = contracts::find_contract_details(&mut client, &contract)?;
///
///     for contract_detail in &results {
///         println!("contract: {:?}", contract_detail);
///     }
///
///     Ok(())
/// }
/// ```
pub fn find_contract_details<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
) -> Result<Vec<ContractDetails>> {
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

    let request_id = client.next_request_id();

    let packet = encode_request_contract_data(client.server_version(), request_id, contract)?;

    info!("outbound message: {:?}", packet);

    let promise = client.send_message(request_id, packet)?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();

    for mut message in promise {
        match message.message_type() {
            IncomingMessage::ContractData => {
                info!("inbound message: {:?}", message);
                let decoded = decode_contract_details(client.server_version(), &mut message)?;
                contract_details.push(decoded);
            }
            IncomingMessage::ContractDataEnd => {
                info!("contract data end: {:?}", message);
                break;
            }
            IncomingMessage::Error => {
                error!("error: {:?}", message);
                return Err(anyhow!("contract_details {:?}", message));
            }
            _ => {
                error!("unexpected message: {:?}", message);
            }
        }
    }

    Ok(contract_details)
}

fn encode_request_contract_data(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
) -> Result<RequestPacket> {
    const VERSION: i32 = 8;

    let mut packet = RequestPacket::default();

    packet.add_field(&OutgoingMessage::RequestContractData);
    packet.add_field(&VERSION);

    if server_version >= server_versions::CONTRACT_DATA_CHAIN {
        packet.add_field(&request_id);
    }

    if server_version >= server_versions::CONTRACT_CONID {
        packet.add_field(&contract.contract_id);
    }

    packet.add_field(&contract.symbol);
    packet.add_field(&contract.security_type);
    packet.add_field(&contract.last_trade_date_or_contract_month);
    packet.add_field(&contract.strike);
    packet.add_field(&contract.right);

    if server_version >= 15 {
        packet.add_field(&contract.multiplier);
    }

    if server_version >= server_versions::PRIMARYEXCH {
        packet.add_field(&contract.exchange);
        packet.add_field(&contract.primary_exchange);
    } else if server_version >= server_versions::LINKING {
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

    if server_version >= server_versions::TRADING_CLASS {
        packet.add_field(&contract.trading_class);
    }
    if server_version >= 31 {
        packet.add_field(&contract.include_expired);
    }
    if server_version >= server_versions::SEC_ID_TYPE {
        packet.add_field(&contract.security_id_type);
        packet.add_field(&contract.security_id);
    }
    if server_version >= server_versions::BOND_ISSUERID {
        packet.add_field(&contract.issuer_id);
    }

    Ok(packet)
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

    let mut request_id = -1;
    if message_version >= 3 {
        request_id = message.next_int()?;
    }

    info!(
        "request_id: {}, server_version: {}, message_version: {}",
        request_id, server_version, message_version
    );

    let mut contract = ContractDetails::default();

    contract.contract.symbol = message.next_string()?;
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, &message.next_string()?, false)?;
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = message.next_string()?;
    contract.contract.currency = message.next_string()?;
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version)
    {
        message.next_int()?; // mdSizeMultiplier no longer used
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
        contract.ev_multiplier = message.next_double()?;
    }
    if message_version >= 7 {
        let sec_id_list_count = message.next_int()?;
        for i in 0..sec_id_list_count {
            let tag = message.next_string()?;
            let value = message.next_string()?;
            contract.sec_id_list.push(TagValue { tag, value });
        }
    }
    if server_version > server_versions::AGG_GROUP {
        contract.agg_group = message.next_int()?;
    }
    if server_version > server_versions::UNDERLYING_INFO {
        contract.under_symbol = message.next_string()?;
        contract.under_security_type = message.next_string()?;
    }
    if server_version > server_versions::MARKET_RULES {
        contract.market_rule_ids = message.next_string()?;
    }
    if server_version > server_versions::REAL_EXPIRATION_DATE {
        contract.real_expiration_date = message.next_string()?;
    }
    if server_version > server_versions::STOCK_TYPE {
        contract.stock_type = message.next_string()?;
    }
    if (server_versions::FRACTIONAL_SIZE_SUPPORT..server_versions::SIZE_RULES)
        .contains(&server_version)
    {
        message.next_double()?; // size min tick -- no longer used
    }
    if server_version >= server_versions::SIZE_RULES {
        contract.min_size = message.next_double()?;
        contract.size_increment = message.next_double()?;
        contract.suggested_size_increment = message.next_double()?;
    }

    Ok(contract)
}

fn read_last_trade_date(
    contract: &mut ContractDetails,
    last_trade_date_or_contract_month: &str,
    is_bond: bool,
) -> Result<()> {
    if last_trade_date_or_contract_month.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = if last_trade_date_or_contract_month.contains('-') {
        last_trade_date_or_contract_month.split('-').collect()
    } else {
        // let re = Regex::new(r"\s+").unwrap();
        last_trade_date_or_contract_month.split(' ').collect()
    };

    if !splitted.is_empty() {
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

/// Contract data and list of derivative security types
#[derive(Debug)]
pub struct ContractDescription {
    pub contract: Contract,
    pub derivative_security_types: Vec<String>,
}

/// Requests matching stock symbols.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::BasicClient;
/// use ibapi::contracts;
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = BasicClient::connect("localhost:4002")?;
///
///     let contracts = contracts::find_contract_descriptions_matching(&mut client, "IB")?;
///
///     for contract in &contracts {
///         println!("contract: {:?}", contract);
///     }
///
///     Ok(())
/// }
/// ```
pub fn find_contract_descriptions_matching<C: Client + Debug>(
    client: &mut C,
    pattern: &str,
) -> Result<Vec<ContractDescription>> {
    client.check_server_version(
        server_versions::REQ_MATCHING_SYMBOLS,
        "It does not support mathing symbols requests.",
    )?;

    let request_id = client.next_request_id();
    let request = encode_request_matching_symbols(request_id, pattern)?;

    let mut promise = client.send_message(request_id, request)?;

    if let Some(mut message) = promise.next() {
        match message.message_type() {
            IncomingMessage::SymbolSamples => {
                return decode_contract_descriptions(client.server_version(), &mut message);
            }
            IncomingMessage::Error => {
                error!("unexpected error: {:?}", message);
                return Err(anyhow!("unexpected error: {:?}", message));
            }
            _ => {
                info!("unexpected message: {:?}", message);
                return Err(anyhow!("unexpected message: {:?}", message));
            }
        }
    }

    Ok(Vec::default())
}

fn encode_request_matching_symbols(request_id: i32, pattern: &str) -> Result<RequestPacket> {
    let mut message = RequestPacket::default();

    message.add_field(&OutgoingMessage::RequestMatchingSymbols);
    message.add_field(&request_id);
    message.add_field(&pattern);

    Ok(message)
}

fn decode_contract_descriptions(
    server_version: i32,
    message: &mut ResponsePacket,
) -> Result<Vec<ContractDescription>> {
    message.skip(); // message type

    let mut contract_descriptions: Vec<ContractDescription> = Vec::default();

    let request_id = message.next_int()?;
    let contract_descriptions_count = message.next_int()?;

    if contract_descriptions_count < 1 {
        return Ok(contract_descriptions);
    }

    for i in 0..contract_descriptions_count {
        let mut contract = Contract {
            contract_id: message.next_int()?,
            symbol: message.next_string()?,
            security_type: SecurityType::from(&message.next_string()?),
            primary_exchange: message.next_string()?,
            currency: message.next_string()?,
            ..Default::default()
        };

        let mut derivative_security_types: Vec<String> = Vec::default();
        let derivative_security_types_count = message.next_int()?;
        for i in 0..derivative_security_types_count {
            derivative_security_types.push(message.next_string()?);
        }

        if server_version >= server_versions::BOND_ISSUERID {
            contract.description = message.next_string()?;
            contract.issuer_id = message.next_string()?;
        }

        contract_descriptions.push(ContractDescription {
            contract,
            derivative_security_types,
        });
    }

    Ok(contract_descriptions)
}
