use crate::{contracts::tick_types::TickType, contracts::SecurityType, messages::ResponseMessage, orders::TagValue, server_versions, Error};

use super::{Contract, ContractDescription, ContractDetails, MarketRule, OptionChain, OptionComputation, PriceIncrement};

#[cfg(test)]
mod tests;

pub(super) fn decode_contract_details(server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails, Error> {
    message.skip(); // message type

    let mut message_version = 8;
    if server_version < server_versions::SIZE_RULES {
        message_version = message.next_int()?;
    }

    if message_version >= 3 {
        // request id
        message.skip();
    }

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
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version) {
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
        for _ in 0..sec_id_list_count {
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
    if (server_versions::FRACTIONAL_SIZE_SUPPORT..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_double()?; // size min tick -- no longer used
    }
    if server_version >= server_versions::SIZE_RULES {
        contract.min_size = message.next_double()?;
        contract.size_increment = message.next_double()?;
        contract.suggested_size_increment = message.next_double()?;
    }

    Ok(contract)
}

fn read_last_trade_date(contract: &mut ContractDetails, last_trade_date_or_contract_month: &str, is_bond: bool) -> Result<(), Error> {
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

pub(super) fn decode_contract_descriptions(server_version: i32, message: &mut ResponseMessage) -> Result<Vec<ContractDescription>, Error> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let contract_descriptions_count = message.next_int()?;

    if contract_descriptions_count < 1 {
        return Ok(Vec::default());
    }

    let mut contract_descriptions: Vec<ContractDescription> = Vec::with_capacity(contract_descriptions_count as usize);

    for _ in 0..contract_descriptions_count {
        let mut contract = Contract {
            contract_id: message.next_int()?,
            symbol: message.next_string()?,
            security_type: SecurityType::from(&message.next_string()?),
            primary_exchange: message.next_string()?,
            currency: message.next_string()?,
            ..Default::default()
        };

        let derivative_security_types_count = message.next_int()?;
        let mut derivative_security_types: Vec<String> = Vec::with_capacity(derivative_security_types_count as usize);
        for _ in 0..derivative_security_types_count {
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

pub(super) fn decode_market_rule(message: &mut ResponseMessage) -> Result<MarketRule, Error> {
    message.skip(); // message type

    let mut market_rule = MarketRule {
        market_rule_id: message.next_int()?,
        ..Default::default()
    };

    let price_increments_count = message.next_int()?;
    for _ in 0..price_increments_count {
        market_rule.price_increments.push(PriceIncrement {
            low_edge: message.next_double()?,
            increment: message.next_double()?,
        });
    }

    Ok(market_rule)
}

pub(crate) fn decode_option_computation(server_version: i32, message: &mut ResponseMessage) -> Result<OptionComputation, Error> {
    message.skip(); // message type

    let message_version = if server_version >= server_versions::PRICE_BASED_VOLATILITY {
        i32::MAX
    } else {
        message.next_int()?
    };

    message.skip(); // request id

    let mut computation = OptionComputation {
        field: TickType::from(message.next_int()?),
        ..Default::default()
    };

    if server_version >= server_versions::PRICE_BASED_VOLATILITY {
        computation.tick_attribute = Some(message.next_int()?);
    }

    computation.implied_volatility = next_optional_double(message, -1.0)?;
    computation.delta = next_optional_double(message, -2.0)?;

    if message_version >= 6 || computation.field == TickType::ModelOption || computation.field == TickType::DelayedModelOption {
        computation.option_price = next_optional_double(message, -1.0)?;
        computation.present_value_dividend = next_optional_double(message, -1.0)?;
    }

    if message_version >= 6 {
        computation.gamma = next_optional_double(message, -2.0)?;
        computation.vega = next_optional_double(message, -2.0)?;
        computation.theta = next_optional_double(message, -2.0)?;
        computation.underlying_price = next_optional_double(message, -1.0)?;
    }

    Ok(computation)
}

fn next_optional_double(message: &mut ResponseMessage, none_value: f64) -> Result<Option<f64>, Error> {
    let value = message.next_double()?;
    if value == none_value {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(super) fn decode_option_chain(message: &mut ResponseMessage) -> Result<OptionChain, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let mut option_chain = OptionChain {
        exchange: message.next_string()?,
        underlying_contract_id: message.next_int()?,
        trading_class: message.next_string()?,
        multiplier: message.next_string()?,
        ..Default::default()
    };

    let expirations_count = message.next_int()?;
    option_chain.expirations.reserve(expirations_count as usize);
    for _ in 0..expirations_count {
        option_chain.expirations.push(message.next_string()?);
    }

    let strikes_count = message.next_int()?;
    option_chain.strikes.reserve(strikes_count as usize);
    for _ in 0..strikes_count {
        option_chain.strikes.push(message.next_double()?);
    }

    Ok(option_chain)
}
