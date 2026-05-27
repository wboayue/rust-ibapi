use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

use crate::contracts::{ContractDescription, ContractDetails, MarketRule, OptionChain, SmartComponent};

// `TickOptionComputation` (msg 21, gate 206 PROTOBUF_MARKET_DATA) is decoded
// in `market_data/realtime/common/decoders` and routed here via the narrow
// re-export below — same proto, same `OptionComputation` struct, no point
// duplicating the decoder.
pub(crate) use crate::market_data::realtime::common::decoders::decode_tick_option_computation;

// All originating outgoing-request gates for ContractData / SymbolSamples /
// MarketRule / SecurityDefinitionOptionParameter are <= the connection floor,
// so the server always emits proto framing for these messages — text-framed
// arrival is rejected via `ResponseMessage::require_proto` and skip-classifies
// (rule 20).

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

// === Protobuf decoders ===

use prost::Message;

use crate::contracts::PriceIncrement;

pub(crate) fn decode_contract_data_proto(bytes: &[u8]) -> Result<ContractDetails, Error> {
    let p: crate::proto::ContractData = Message::decode(bytes)?;
    let default_contract = crate::proto::Contract::default();
    let default_details = crate::proto::ContractDetails::default();
    let proto_contract = p.contract.as_ref().unwrap_or(&default_contract);
    let proto_details = p.contract_details.as_ref().unwrap_or(&default_details);
    crate::proto::decoders::decode_contract_details(proto_contract, proto_details)
}

pub(crate) fn decode_symbol_samples_proto(bytes: &[u8]) -> Result<Vec<ContractDescription>, Error> {
    let p: crate::proto::SymbolSamples = Message::decode(bytes)?;
    p.contract_descriptions
        .into_iter()
        .map(|d| {
            let contract = d
                .contract
                .as_ref()
                .map(crate::proto::decoders::decode_contract)
                .transpose()?
                .unwrap_or_default();
            Ok(ContractDescription {
                contract,
                derivative_security_types: d.derivative_sec_types,
            })
        })
        .collect()
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

pub(crate) fn decode_smart_components(message: &ResponseMessage) -> Result<Vec<SmartComponent>, Error> {
    decode_smart_components_proto(message.require_proto()?)
}

pub(crate) fn decode_smart_components_proto(bytes: &[u8]) -> Result<Vec<SmartComponent>, Error> {
    let p: crate::proto::SmartComponents = Message::decode(bytes)?;
    Ok(p.smart_components
        .into_iter()
        .map(|c| SmartComponent {
            bit_number: c.bit_number.unwrap_or_default(),
            exchange: c.exchange.unwrap_or_default(),
            exchange_letter: c.exchange_letter.unwrap_or_default(),
        })
        .collect())
}

pub(in crate::contracts) fn decode_smart_components_message(message: &ResponseMessage) -> Result<Vec<SmartComponent>, Error> {
    match message.message_type() {
        IncomingMessages::SmartComponents => decode_smart_components(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

#[cfg(test)]
mod tests;
