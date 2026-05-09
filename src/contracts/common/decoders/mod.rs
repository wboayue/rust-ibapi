use crate::{contracts::tick_types::TickType, messages::ResponseMessage, server_versions, Error};

use crate::contracts::{ContractDescription, ContractDetails, MarketRule, OptionChain, OptionComputation};

// All originating outgoing-request gates for ContractData / SymbolSamples /
// MarketRule / SecurityDefinitionOptionParameter are <= the connection floor
// (`PROTOBUF_SCAN_DATA` = 210), so the server always emits proto framing for
// these messages — text-framed arrival is rejected via
// `ResponseMessage::require_proto` and skip-classifies (rule 20).

pub(in crate::contracts) fn decode_contract_details(_server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails, Error> {
    decode_contract_data_proto(message.require_proto()?)
}

pub(in crate::contracts) fn decode_contract_descriptions(
    _server_version: i32,
    message: &mut ResponseMessage,
) -> Result<Vec<ContractDescription>, Error> {
    decode_symbol_samples_proto(message.require_proto()?)
}

pub(in crate::contracts) fn decode_market_rule(message: &mut ResponseMessage) -> Result<MarketRule, Error> {
    decode_market_rule_proto(message.require_proto()?)
}

pub(in crate::contracts) fn decode_option_chain(message: &mut ResponseMessage) -> Result<OptionChain, Error> {
    decode_option_chain_proto(message.require_proto()?)
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

// === Protobuf decoders ===

use prost::Message;

use crate::contracts::PriceIncrement;

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
