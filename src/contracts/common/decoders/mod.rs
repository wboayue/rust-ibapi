use time::macros::format_description;
use time::Date;

use crate::{
    contracts::tick_types::TickType,
    contracts::{Currency, Exchange, SecurityType, Symbol},
    messages::ResponseMessage,
    server_versions, Error,
};

use crate::contracts::{
    Contract, ContractDescription, ContractDetails, FundAssetType, FundDistributionPolicyIndicator, IneligibilityReason, MarketRule, OptionChain,
    OptionComputation, PriceIncrement, TagValue,
};

pub(in crate::contracts) fn decode_contract_details(server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails, Error> {
    message.decode_proto_or_text(decode_contract_data_proto, |msg| decode_contract_details_text(server_version, msg))
}

fn decode_contract_details_text(server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails, Error> {
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

    contract.contract.symbol = Symbol::from(message.next_string()?);
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, &message.next_string()?, false)?;
    if server_version >= server_versions::LAST_TRADE_DATE {
        let last_trade_date_str = message.next_string()?;
        if !last_trade_date_str.is_empty() {
            let fmt = format_description!("[year][month][day]");
            contract.contract.last_trade_date = Date::parse(&last_trade_date_str, fmt).ok();
        }
    }
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = Exchange::from(message.next_string()?);
    contract.contract.currency = Currency::from(message.next_string()?);
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_int()?; // mdSizeMultiplier no longer used
    }
    contract.contract.multiplier = message.next_string()?;
    contract.order_types = split_to_vec(&message.next_string()?);
    contract.valid_exchanges = split_to_vec(&message.next_string()?);
    if message_version >= 2 {
        contract.price_magnifier = message.next_int()?;
    }
    if message_version >= 4 {
        contract.under_contract_id = message.next_int()?;
    }
    if message_version >= 5 {
        //        https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EDecoder.cs#L1626
        contract.long_name = message.next_string()?;
        contract.contract.primary_exchange = Exchange::from(message.next_string()?);
    }
    if message_version >= 6 {
        contract.contract_month = message.next_string()?;
        contract.industry = message.next_string()?;
        contract.category = message.next_string()?;
        contract.subcategory = message.next_string()?;
        contract.time_zone_id = message.next_string()?;
        contract.trading_hours = split_hours(&message.next_string()?);
        contract.liquid_hours = split_hours(&message.next_string()?);
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
        contract.market_rule_ids = split_to_vec(&message.next_string()?);
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
    if server_version >= server_versions::FUND_DATA_FIELDS && contract.contract.security_type == SecurityType::MutualFund {
        contract.fund_name = message.next_string()?;
        contract.fund_family = message.next_string()?;
        contract.fund_type = message.next_string()?;
        contract.fund_front_load = message.next_string()?;
        contract.fund_back_load = message.next_string()?;
        contract.fund_back_load_time_interval = message.next_string()?;
        contract.fund_management_fee = message.next_string()?;
        contract.fund_closed = message.next_bool()?;
        contract.fund_closed_for_new_investors = message.next_bool()?;
        contract.fund_closed_for_new_money = message.next_bool()?;
        contract.fund_notify_amount = message.next_string()?;
        contract.fund_minimum_initial_purchase = message.next_string()?;
        contract.fund_subsequent_minimum_purchase = message.next_string()?;
        contract.fund_blue_sky_states = message.next_string()?;
        contract.fund_blue_sky_territories = message.next_string()?;
        contract.fund_distribution_policy_indicator = FundDistributionPolicyIndicator::from(message.next_string()?.as_str());
        contract.fund_asset_type = FundAssetType::from(message.next_string()?.as_str());
    }
    if server_version >= server_versions::INELIGIBILITY_REASONS {
        let count = message.next_int()?;
        for _ in 0..count {
            contract.ineligibility_reasons.push(IneligibilityReason {
                id: message.next_string()?,
                description: message.next_string()?,
            });
        }
    }

    Ok(contract)
}

fn split_hours(hours: &str) -> Vec<String> {
    hours.split(";").map(|s| s.to_string()).collect()
}

fn split_to_vec(s: &str) -> Vec<String> {
    s.split(",").map(|s| s.to_string()).collect()
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

pub(in crate::contracts) fn decode_contract_descriptions(
    server_version: i32,
    message: &mut ResponseMessage,
) -> Result<Vec<ContractDescription>, Error> {
    message.decode_proto_or_text(decode_symbol_samples_proto, |msg| {
        msg.skip(); // message type

        let _request_id = msg.next_int()?;
        let contract_descriptions_count = msg.next_int()?;

        if contract_descriptions_count < 1 {
            return Ok(Vec::default());
        }

        let mut contract_descriptions: Vec<ContractDescription> = Vec::with_capacity(contract_descriptions_count as usize);

        for _ in 0..contract_descriptions_count {
            let mut contract = Contract {
                contract_id: msg.next_int()?,
                symbol: Symbol::from(msg.next_string()?),
                security_type: SecurityType::from(&msg.next_string()?),
                primary_exchange: Exchange::from(msg.next_string()?),
                currency: Currency::from(msg.next_string()?),
                ..Default::default()
            };

            let derivative_security_types_count = msg.next_int()?;
            let mut derivative_security_types: Vec<String> = Vec::with_capacity(derivative_security_types_count as usize);
            for _ in 0..derivative_security_types_count {
                derivative_security_types.push(msg.next_string()?);
            }

            if server_version >= server_versions::BOND_ISSUERID {
                contract.description = msg.next_string()?;
                contract.issuer_id = msg.next_string()?;
            }

            contract_descriptions.push(ContractDescription {
                contract,
                derivative_security_types,
            });
        }

        Ok(contract_descriptions)
    })
}

pub(in crate::contracts) fn decode_market_rule(message: &mut ResponseMessage) -> Result<MarketRule, Error> {
    message.decode_proto_or_text(decode_market_rule_proto, |msg| {
        msg.skip(); // message type

        let mut market_rule = MarketRule {
            market_rule_id: msg.next_int()?,
            ..Default::default()
        };

        let price_increments_count = msg.next_int()?;
        for _ in 0..price_increments_count {
            market_rule.price_increments.push(PriceIncrement {
                low_edge: msg.next_double()?,
                increment: msg.next_double()?,
            });
        }

        Ok(market_rule)
    })
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

pub(in crate::contracts) fn decode_option_chain(message: &mut ResponseMessage) -> Result<OptionChain, Error> {
    message.decode_proto_or_text(decode_option_chain_proto, |msg| {
        msg.skip(); // message type
        msg.skip(); // request id

        let mut option_chain = OptionChain {
            exchange: msg.next_string()?,
            underlying_contract_id: msg.next_int()?,
            trading_class: msg.next_string()?,
            multiplier: msg.next_string()?,
            ..Default::default()
        };

        let expirations_count = msg.next_int()?;
        option_chain.expirations.reserve(expirations_count as usize);
        for _ in 0..expirations_count {
            option_chain.expirations.push(msg.next_string()?);
        }

        let strikes_count = msg.next_int()?;
        option_chain.strikes.reserve(strikes_count as usize);
        for _ in 0..strikes_count {
            option_chain.strikes.push(msg.next_double()?);
        }

        Ok(option_chain)
    })
}

// === Protobuf decoders ===

use prost::Message;

pub(crate) fn decode_contract_data_proto(bytes: &[u8]) -> Result<ContractDetails, Error> {
    let p: crate::proto::ContractData = Message::decode(bytes)?;
    let default_contract = crate::proto::Contract::default();
    let default_details = crate::proto::ContractDetails::default();
    let proto_contract = p.contract.as_ref().unwrap_or(&default_contract);
    let proto_details = p.contract_details.as_ref().unwrap_or(&default_details);
    Ok(crate::proto::decoders::decode_contract_details(proto_contract, proto_details))
}

pub(crate) fn decode_symbol_samples_proto(bytes: &[u8]) -> Result<Vec<ContractDescription>, Error> {
    let p: crate::proto::SymbolSamples = Message::decode(bytes)?;
    Ok(p.contract_descriptions
        .into_iter()
        .map(|d| {
            let contract = d.contract.as_ref().map(crate::proto::decoders::decode_contract).unwrap_or_default();
            ContractDescription {
                contract,
                derivative_security_types: d.derivative_sec_types,
            }
        })
        .collect())
}

pub(crate) fn decode_market_rule_proto(bytes: &[u8]) -> Result<MarketRule, Error> {
    let p: crate::proto::MarketRule = Message::decode(bytes)?;
    Ok(MarketRule {
        market_rule_id: p.market_rule_id.unwrap_or_default(),
        price_increments: p
            .price_increments
            .into_iter()
            .map(|pi| PriceIncrement {
                low_edge: pi.low_edge.unwrap_or_default(),
                increment: pi.increment.unwrap_or_default(),
            })
            .collect(),
    })
}

pub(crate) fn decode_option_chain_proto(bytes: &[u8]) -> Result<OptionChain, Error> {
    let p: crate::proto::SecDefOptParameter = Message::decode(bytes)?;
    Ok(OptionChain {
        exchange: p.exchange.unwrap_or_default(),
        underlying_contract_id: p.underlying_con_id.unwrap_or_default(),
        trading_class: p.trading_class.unwrap_or_default(),
        multiplier: p.multiplier.unwrap_or_default(),
        expirations: p.expirations,
        strikes: p.strikes,
    })
}

#[cfg(test)]
mod tests;
