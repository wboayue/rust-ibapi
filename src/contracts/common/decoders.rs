use crate::messages::ResponseMessage;
use crate::Error;

use crate::contracts::{
    ContractDescription, ContractDetails, FundAssetType, FundDistributionPolicyIndicator, IneligibilityReason, MarketRule, OptionChain,
    SmartComponent,
};
use crate::proto;
use crate::proto::decoders::{decode_contract, parse_decimal_or_zero, parse_optional_decimal, s, tag_values};

// `TickOptionComputation` (msg 21, gate 206 PROTOBUF_MARKET_DATA) is decoded
// in `market_data/realtime/common/decoders` and routed here via the narrow
// re-export below — same proto, same `OptionComputation` struct, no point
// duplicating the decoder.
pub(crate) use crate::market_data::realtime::common::decode_tick_option_computation;

// All originating outgoing-request gates for ContractData / SymbolSamples /
// MarketRule / SecurityDefinitionOptionParameter are <= the connection floor,
// so the server always emits proto framing for these messages — text-framed
// arrival is rejected via `ResponseMessage::require_proto`, which raises
// `Error::UnexpectedWireFormat` (docs/rules/wire/proto-only-decoding.md).

pub(in crate::contracts) fn decode_contract_details(message: &ResponseMessage) -> Result<ContractDetails, Error> {
    decode_contract_data_proto(message.proto()?)
}

/// TWS answers a bond query with `BondContractData` (msg 18): the same
/// `ContractData` proto as other securities, decoded with C#'s bond-only
/// handling of the last trade date (`EDecoderUtils.SetLastTradeDate`).
pub(in crate::contracts) fn decode_bond_contract_details(message: &ResponseMessage) -> Result<ContractDetails, Error> {
    let mut details = decode_contract_data_proto(message.proto()?)?;
    split_bond_last_trade_date(&mut details);
    Ok(details)
}

/// A bond's `last_trade_date_or_contract_month` carries `maturity [time [zone]]`,
/// split on `-` if present, otherwise on whitespace. The contract field itself
/// is left as sent, as C# does for bonds.
fn split_bond_last_trade_date(details: &mut ContractDetails) {
    let raw = &details.contract.last_trade_date_or_contract_month;
    let parts: Vec<String> = if raw.contains('-') {
        raw.split('-').map(str::to_string).collect()
    } else {
        raw.split_whitespace().map(str::to_string).collect()
    };
    if let Some(maturity) = parts.first() {
        details.maturity = maturity.clone();
    }
    if let Some(time) = parts.get(1) {
        details.last_trade_time = time.clone();
    }
    if let Some(zone) = parts.get(2) {
        details.time_zone_id = zone.clone();
    }
}

pub(in crate::contracts) fn decode_option_chain(message: &ResponseMessage) -> Result<OptionChain, Error> {
    decode_option_chain_proto(message.proto()?)
}

// === Protobuf decoders ===

use crate::contracts::PriceIncrement;

pub(crate) fn decode_contract_data_proto(p: crate::proto::ContractData) -> Result<ContractDetails, Error> {
    let proto_contract = crate::proto::decoders::required(p.contract.as_ref(), "contract", "ContractData")?;
    let proto_details = crate::proto::decoders::required(p.contract_details.as_ref(), "contract_details", "ContractData")?;
    decode_contract_and_details(proto_contract, proto_details)
}

pub(crate) fn decode_symbol_samples_proto(p: crate::proto::SymbolSamples) -> Result<Vec<ContractDescription>, Error> {
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

/// The `MarketRule` processor for `market_rule(market_rule_id)`. TWS answers
/// on the shared channel with no request id, so a frame for another rule id
/// is a concurrent `market_rule`'s answer: `Ok(None)`, and the caller reads on.
pub(crate) fn market_rule_for(market_rule_id: i32) -> impl Fn(&ResponseMessage) -> Result<Option<MarketRule>, Error> {
    let decode = crate::common::request_helpers::expect_proto(decode_market_rule_proto);
    move |message| {
        let rule = decode(message)?;
        Ok((rule.market_rule_id == market_rule_id).then_some(rule))
    }
}

pub(crate) fn decode_market_rule_proto(p: crate::proto::MarketRule) -> Result<MarketRule, Error> {
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

pub(crate) fn decode_option_chain_proto(p: crate::proto::SecDefOptParameter) -> Result<OptionChain, Error> {
    Ok(OptionChain {
        exchange: p.exchange.unwrap_or_default(),
        underlying_contract_id: p.underlying_con_id.unwrap_or_default(),
        trading_class: p.trading_class.unwrap_or_default(),
        multiplier: p.multiplier.unwrap_or_default(),
        expirations: p.expirations,
        strikes: p.strikes,
    })
}

pub(crate) fn decode_smart_components_proto(p: crate::proto::SmartComponents) -> Result<Vec<SmartComponent>, Error> {
    Ok(p.smart_components
        .into_iter()
        .map(|c| SmartComponent {
            bit_number: c.bit_number.unwrap_or_default(),
            exchange: c.exchange.unwrap_or_default(),
            exchange_letter: c.exchange_letter.unwrap_or_default(),
        })
        .collect())
}

// === Sub-message decoders (shared by the frame decoders above) ===

pub(crate) fn decode_contract_and_details(
    proto_contract: &proto::Contract,
    proto_details: &proto::ContractDetails,
) -> Result<ContractDetails, Error> {
    let contract = decode_contract(proto_contract)?;

    Ok(ContractDetails {
        contract,
        market_name: s(&proto_details.market_name),
        // min_tick is StringToDoubleMax upstream, not StringToDecimal; the extra
        // integer sentinels are unreachable here (no price tick is 2147483647).
        min_tick: parse_decimal_or_zero(proto_details.min_tick.as_deref())?,
        order_types: proto_details
            .order_types
            .as_deref()
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default(),
        valid_exchanges: proto_details
            .valid_exchanges
            .as_deref()
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default(),
        price_magnifier: proto_details.price_magnifier.unwrap_or_default(),
        under_contract_id: proto_details.under_con_id.unwrap_or_default(),
        long_name: s(&proto_details.long_name),
        contract_month: s(&proto_details.contract_month),
        industry: s(&proto_details.industry),
        category: s(&proto_details.category),
        subcategory: s(&proto_details.subcategory),
        time_zone_id: s(&proto_details.time_zone_id),
        trading_hours: proto_details
            .trading_hours
            .as_deref()
            .map(|s| s.split(';').map(|t| t.to_string()).collect())
            .unwrap_or_default(),
        liquid_hours: proto_details
            .liquid_hours
            .as_deref()
            .map(|s| s.split(';').map(|t| t.to_string()).collect())
            .unwrap_or_default(),
        ev_rule: s(&proto_details.ev_rule),
        ev_multiplier: proto_details.ev_multiplier.unwrap_or_default(),
        agg_group: proto_details.agg_group.unwrap_or_default(),
        sec_id_list: tag_values(&proto_details.sec_id_list),
        under_symbol: s(&proto_details.under_symbol),
        under_security_type: s(&proto_details.under_sec_type),
        market_rule_ids: proto_details
            .market_rule_ids
            .as_deref()
            .map(|s| s.split(',').map(|t| t.to_string()).collect())
            .unwrap_or_default(),
        real_expiration_date: s(&proto_details.real_expiration_date),
        stock_type: s(&proto_details.stock_type),
        settlement_method: s(&proto_details.settlement_method),
        // Absent means unset upstream (ContractDetails.cs ctor sets decimal.MaxValue),
        // and contracts without size rules really do omit these.
        min_size: parse_optional_decimal(proto_details.min_size.as_deref())?,
        size_increment: parse_optional_decimal(proto_details.size_increment.as_deref())?,
        suggested_size_increment: parse_optional_decimal(proto_details.suggested_size_increment.as_deref())?,
        // fund fields
        fund_name: s(&proto_details.fund_name),
        fund_family: s(&proto_details.fund_family),
        fund_type: s(&proto_details.fund_type),
        fund_front_load: s(&proto_details.fund_front_load),
        fund_back_load: s(&proto_details.fund_back_load),
        fund_back_load_time_interval: s(&proto_details.fund_back_load_time_interval),
        fund_management_fee: s(&proto_details.fund_management_fee),
        fund_closed: proto_details.fund_closed.unwrap_or_default(),
        fund_closed_for_new_investors: proto_details.fund_closed_for_new_investors.unwrap_or_default(),
        fund_closed_for_new_money: proto_details.fund_closed_for_new_money.unwrap_or_default(),
        fund_notify_amount: s(&proto_details.fund_notify_amount),
        fund_minimum_initial_purchase: s(&proto_details.fund_minimum_initial_purchase),
        fund_subsequent_minimum_purchase: s(&proto_details.fund_minimum_subsequent_purchase),
        fund_blue_sky_states: s(&proto_details.fund_blue_sky_states),
        fund_blue_sky_territories: s(&proto_details.fund_blue_sky_territories),
        fund_distribution_policy_indicator: FundDistributionPolicyIndicator::from(
            proto_details.fund_distribution_policy_indicator.as_deref().unwrap_or(""),
        ),
        fund_asset_type: FundAssetType::from(proto_details.fund_asset_type.as_deref().unwrap_or("")),
        // bond fields
        cusip: s(&proto_details.cusip),
        ratings: s(&proto_details.ratings),
        desc_append: s(&proto_details.desc_append),
        bond_type: s(&proto_details.bond_type),
        coupon_type: s(&proto_details.coupon_type),
        callable: proto_details.callable.unwrap_or_default(),
        putable: proto_details.puttable.unwrap_or_default(),
        coupon: proto_details.coupon.unwrap_or_default(),
        convertible: proto_details.convertible.unwrap_or_default(),
        maturity: String::new(),
        issue_date: s(&proto_details.issue_date),
        next_option_date: s(&proto_details.next_option_date),
        next_option_type: s(&proto_details.next_option_type),
        next_option_partial: proto_details.next_option_partial.unwrap_or_default(),
        notes: s(&proto_details.bond_notes),
        // ineligibility reasons
        ineligibility_reasons: proto_details
            .ineligibility_reason_list
            .iter()
            .map(|r| IneligibilityReason {
                id: s(&r.id),
                description: s(&r.description),
            })
            .collect(),
        // defaults for fields not in protobuf
        last_trade_time: String::new(),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
