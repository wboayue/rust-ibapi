use crate::contracts::{
    ComboLeg, ComboLegOpenClose, Contract, ContractDetails, Currency, DeltaNeutralContract, Exchange, FundAssetType, FundDistributionPolicyIndicator,
    IneligibilityReason, LegAction, SecurityIdType, SecurityType, Symbol, TagValue,
};
use crate::orders::SoftDollarTier;
use crate::proto;
use crate::Error;

// === Helper functions ===

/// Decode a protobuf message, mapping a failure to [`Error::ProtobufDecode`].
/// Use this instead of `prost::Message::decode(..)?`: `Error` has no public
/// `From<prost::DecodeError>`, so prost upgrades stay out of the crate's API.
pub(crate) trait DecodeProto: prost::Message + Default {
    fn decode_proto(buf: &[u8]) -> Result<Self, Error> {
        Self::decode(buf).map_err(Error::protobuf_decode)
    }
}

impl<M: prost::Message + Default> DecodeProto for M {}

pub(crate) fn s(opt: &Option<String>) -> String {
    opt.clone().unwrap_or_default()
}

pub(crate) fn optional_f64(val: Option<f64>) -> Option<f64> {
    val.filter(|&v| v != f64::MAX)
}

// === Decimal wire fields (protobuf `optional string`) ===
//
// IBKR ships sizes, quantities and volumes as decimal strings. The helpers below
// are the only sanctioned way to read one; they are the numeric counterpart to
// the `parse_required` / `parse_optional` pair, which handle enumerated fields.

/// Integer sentinels TWS uses to mean "no value" on a decimal-typed field.
///
/// These are the ones that must be matched *literally*, because they parse to
/// perfectly ordinary finite numbers — `"2147483647"` is a sentinel but
/// `"2147483647.0"` is a real size and must survive. The float sentinel
/// (`f64::MAX`) is deliberately not here; [`parse_optional_decimal`] catches it
/// numerically after the parse, which covers every spelling at once.
///
/// Mirrors `Util.StringToDecimal` in the C# reference client
/// (tws-api `source/csharpclient/client/Util.cs`), which maps each of these to
/// `decimal.MaxValue` — upstream's "unset" marker.
const UNSET_DECIMAL_WIRE: [&str; 3] = [
    "9223372036854775807",  // i64::MAX
    "2147483647",           // i32::MAX
    "-9223372036854775808", // i64::MIN
];

/// Parse an optional decimal-typed wire field.
///
/// - `Ok(None)` — field absent, present-but-empty, or an "unset" marker: either a
///   literal [`UNSET_DECIMAL_WIRE`] entry or a value that parses to `f64::MAX`,
///   infinity, or NaN. All mean "TWS carries no value here".
/// - `Ok(Some(v))` — a parsed value. Fractional sizes survive: `"0.5"` → `0.5`.
/// - `Err(Error::Parse)` — non-empty, non-sentinel, and not a number.
///
/// The error arm is the point of the helper. Its predecessors ended in
/// `unwrap_or_default()`, so a wire value of `"0.5"` failed `parse::<i32>()` and
/// silently became `0` — data loss on fractional (crypto) sizes, issue #716.
/// Per docs/rules/wire/enum-typing.md, a decoder rejects malformed input rather
/// than defaulting.
///
/// Whitespace is not trimmed: `Some(" ")` is an error, matching C#'s
/// `decimal.Parse(" ")`, which throws.
///
/// Takes `Option<&str>` to match [`parse_required`] / [`parse_optional`]; proto
/// callers pass `proto.field.as_deref()`.
pub(crate) fn parse_optional_decimal(opt: Option<&str>) -> Result<Option<f64>, Error> {
    let Some(text) = opt.filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if UNSET_DECIMAL_WIRE.contains(&text) {
        return Ok(None);
    }
    let value = text
        .parse::<f64>()
        .map_err(|e| Error::parse_field(text, format!("invalid decimal wire value: {e}")))?;
    // Spelling-independent half of the unset check: catches every rendering of
    // the f64::MAX sentinel ("1.7976931348623157E308", "…e308", "…E+308") and
    // absorbs inf/NaN, which f64::from_str accepts but no size can be.
    if !value.is_finite() || value == f64::MAX {
        return Ok(None);
    }
    Ok(Some(value))
}

/// [`parse_optional_decimal`], collapsing "no value" to `0.0`.
///
/// For fields whose public type is plain `f64` and so cannot represent "unset".
/// `0.0` is what those fields produced for an absent or empty wire value before
/// [`parse_optional_decimal`] existed, so this preserves that behaviour while
/// still erroring on malformed input.
///
/// **Transitional.** The fields whose absent value was most clearly reachable and
/// meaningful — the historical tick and histogram sizes, and
/// `ContractDetails::{min_size, size_increment, suggested_size_increment}` — are
/// already `Option<f64>` and use [`parse_optional_decimal`]. The remaining call
/// sites are not all provably safe: the C# reference client guards several of them
/// with `HasX ? Util.StringToDecimal(..) : decimal.MaxValue`, so upstream models
/// them as "absent means unset" too. `Bar::volume` / `Bar::wap` are the clearest
/// case — MIDPOINT, BID and ASK bars carry no volume, and `EDecoderUtils.cs`
/// defaults them to `decimal.MaxValue` while defaulting open/high/low/close to `0`
/// in the same function. Those are candidates for `Option<f64>` in the follow-up
/// decimal-quantity work; grep this name for the worklist.
pub(crate) fn parse_decimal_or_zero(opt: Option<&str>) -> Result<f64, Error> {
    Ok(parse_optional_decimal(opt)?.unwrap_or(0.0))
}

/// Parse a required enumerated wire field. Both `None` and empty strings
/// surface as `Error::Parse`: an empty wire on a required field is a TWS
/// protocol bug, not a default. Silent fallback to `T::default()` masks
/// incomplete responses (docs/rules/wire/enum-typing.md).
///
/// Accepts `Option<&str>` so proto callers can pass `proto.field.as_deref()`
/// and text-protocol callers can pass `Some(text_str.as_str())` without
/// allocating a wrapper.
pub(crate) fn parse_required<T>(opt: Option<&str>, label: &str) -> Result<T, Error>
where
    T: std::str::FromStr<Err = Error>,
{
    match opt {
        Some(s) if !s.is_empty() => s.parse(),
        _ => Err(Error::Parse(0, String::new(), format!("missing {label}"))),
    }
}

/// Parse an optional enumerated wire field. `None` and `Some("")` both mean
/// "no value" — a valid wire state for fields like `Contract.right` on
/// non-option contracts. Only an unknown non-empty string is an error.
///
/// Accepts `Option<&str>` so proto callers can pass `proto.field.as_deref()`
/// and text-protocol callers can pass `Some(text_str.as_str())` without
/// allocating a wrapper.
pub(crate) fn parse_optional<T>(opt: Option<&str>) -> Result<Option<T>, Error>
where
    T: std::str::FromStr<Err = Error>,
{
    opt.filter(|s| !s.is_empty()).map(|s| s.parse()).transpose()
}

/// A proto submessage or scalar the frame is meaningless without.
///
/// Use only where the reference client drops the whole frame when it is absent
/// rather than synthesizing a default — e.g. `EDecoder.cs` returns before
/// `eWrapper.openOrder(..)` if `Contract`, `Order` or `OrderState` is null, and
/// before `eWrapper.contractDetails(..)` if `Contract` or `ContractDetails` is.
/// This crate has no "skip this frame" channel, so it surfaces the malformed
/// frame as `Error::Parse` instead. Defaulting is the one option neither client
/// takes: it hands the caller data that reads as real, such as a phantom BUY
/// order over an empty contract (docs/rules/wire/enum-typing.md).
pub(crate) fn required<T>(field: Option<T>, name: &str, message: &str) -> Result<T, Error> {
    field.ok_or_else(|| Error::parse_proto(name, format!("missing in {message}")))
}

pub(crate) fn ts(secs: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(secs).unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
}

pub(crate) fn tag_values(map: &std::collections::HashMap<String, String>) -> Vec<TagValue> {
    map.iter()
        .map(|(k, v)| TagValue {
            tag: k.clone(),
            value: v.clone(),
        })
        .collect()
}

// === Shared converters ===

pub fn decode_contract(proto: &proto::Contract) -> Result<Contract, Error> {
    Ok(Contract {
        contract_id: proto.con_id.unwrap_or_default(),
        symbol: Symbol::from(s(&proto.symbol)),
        security_type: parse_optional(proto.sec_type.as_deref())?.unwrap_or_else(|| SecurityType::Other(String::new())),
        last_trade_date_or_contract_month: s(&proto.last_trade_date_or_contract_month),
        strike: proto.strike.unwrap_or_default(),
        right: parse_optional(proto.right.as_deref())?,
        multiplier: proto.multiplier.map(|m| m.to_string()).unwrap_or_default(),
        exchange: Exchange::from(s(&proto.exchange)),
        primary_exchange: Exchange::from(s(&proto.primary_exch)),
        currency: Currency::from(s(&proto.currency)),
        local_symbol: s(&proto.local_symbol),
        trading_class: s(&proto.trading_class),
        include_expired: proto.include_expired.unwrap_or_default(),
        security_id_type: parse_optional::<SecurityIdType>(proto.sec_id_type.as_deref())?,
        security_id: s(&proto.sec_id),
        description: s(&proto.description),
        issuer_id: s(&proto.issuer_id),
        combo_legs_description: s(&proto.combo_legs_descrip),
        combo_legs: proto.combo_legs.iter().map(decode_combo_leg).collect::<Result<Vec<_>, _>>()?,
        delta_neutral_contract: proto.delta_neutral_contract.as_ref().map(decode_delta_neutral_contract),
        last_trade_date: None,
    })
}

pub fn decode_combo_leg(proto: &proto::ComboLeg) -> Result<ComboLeg, Error> {
    Ok(ComboLeg {
        contract_id: proto.con_id.unwrap_or_default(),
        ratio: proto.ratio.unwrap_or_default(),
        action: parse_required::<LegAction>(proto.action.as_deref(), "LegAction")?,
        exchange: s(&proto.exchange),
        open_close: ComboLegOpenClose::from(proto.open_close.unwrap_or_default()),
        short_sale_slot: proto.short_sales_slot.unwrap_or_default(),
        designated_location: s(&proto.designated_location),
        exempt_code: proto.exempt_code.unwrap_or_default(),
    })
}

pub fn decode_delta_neutral_contract(proto: &proto::DeltaNeutralContract) -> DeltaNeutralContract {
    DeltaNeutralContract {
        contract_id: proto.con_id.unwrap_or_default(),
        delta: proto.delta.unwrap_or_default(),
        price: proto.price.unwrap_or_default(),
    }
}

pub fn decode_soft_dollar_tier(proto: &proto::SoftDollarTier) -> SoftDollarTier {
    SoftDollarTier {
        name: s(&proto.name),
        value: s(&proto.value),
        display_name: s(&proto.display_name),
    }
}

pub fn decode_contract_details(proto_contract: &proto::Contract, proto_details: &proto::ContractDetails) -> Result<ContractDetails, Error> {
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
