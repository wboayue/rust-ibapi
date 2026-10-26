//! Shared domain→proto converters for outbound protobuf encoding.

use crate::contracts::{self, Contract};
use crate::orders::Order;
use crate::proto;

/// Encode a cancel-by-request-ID protobuf message.
/// All cancel proto types with a single `req_id` field share this pattern.
macro_rules! encode_cancel_by_id {
    ($request_id:expr, $proto_type:ident, $msg_id:expr) => {{
        use prost::Message;
        let request = crate::proto::$proto_type { req_id: Some($request_id) };
        Ok(crate::messages::encode_protobuf_message($msg_id as i32, &request.encode_to_vec()))
    }};
}
pub(crate) use encode_cancel_by_id;

/// Encode an empty (no-field) protobuf request message.
macro_rules! encode_empty_proto {
    ($proto_type:ident, $msg_id:expr) => {{
        use prost::Message;
        let request = crate::proto::$proto_type {};
        Ok(crate::messages::encode_protobuf_message($msg_id as i32, &request.encode_to_vec()))
    }};
}
pub(crate) use encode_empty_proto;

// === Helper: set Some only for non-empty strings ===

pub(crate) fn some_str(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

pub(crate) fn some_i32_ne(v: i32, default: i32) -> Option<i32> {
    if v == default {
        None
    } else {
        Some(v)
    }
}

pub(crate) fn some_f64_ne(v: f64, default: f64) -> Option<f64> {
    if (v - default).abs() < f64::EPSILON {
        None
    } else {
        Some(v)
    }
}

pub(crate) fn some_i64_ne(v: i64, default: i64) -> Option<i64> {
    if v == default {
        None
    } else {
        Some(v)
    }
}

pub(crate) fn some_bool(v: bool) -> Option<bool> {
    if v {
        Some(true)
    } else {
        None
    }
}

/// Map `Option<&T: Display>` to its wire string. `None` stays `None`; unlike
/// `some_str`, no empty-string filtering — `Option::None` already carries the
/// no-value state at the type level for typed wire enums.
pub(crate) fn some_display<T: std::fmt::Display>(opt: Option<&T>) -> Option<String> {
    opt.map(|v| v.to_string())
}

// === Contract ===

pub fn encode_contract(contract: &Contract) -> proto::Contract {
    encode_contract_with_order(contract, None)
}

pub fn encode_contract_with_order(contract: &Contract, order: Option<&Order>) -> proto::Contract {
    proto::Contract {
        con_id: some_i32_ne(contract.contract_id, 0),
        symbol: some_str(&contract.symbol.to_string()),
        sec_type: some_str(&contract.security_type.to_string()),
        last_trade_date_or_contract_month: some_str(&contract.last_trade_date_or_contract_month),
        strike: some_f64_ne(contract.strike, 0.0),
        right: some_display(contract.right.as_ref()),
        multiplier: contract.multiplier.parse::<f64>().ok(),
        exchange: some_str(&contract.exchange.to_string()),
        primary_exch: some_str(&contract.primary_exchange.to_string()),
        currency: some_str(&contract.currency.to_string()),
        local_symbol: some_str(&contract.local_symbol),
        trading_class: some_str(&contract.trading_class),
        sec_id_type: some_display(contract.security_id_type.as_ref()),
        sec_id: some_str(&contract.security_id),
        description: some_str(&contract.description),
        issuer_id: some_str(&contract.issuer_id),
        include_expired: some_bool(contract.include_expired),
        combo_legs_descrip: some_str(&contract.combo_legs_description),
        delta_neutral_contract: contract.delta_neutral_contract.as_ref().map(encode_delta_neutral_contract),
        combo_legs: encode_combo_legs(contract, order),
        last_trade_date: None,
    }
}

fn encode_delta_neutral_contract(dnc: &contracts::DeltaNeutralContract) -> proto::DeltaNeutralContract {
    proto::DeltaNeutralContract {
        con_id: some_i32_ne(dnc.contract_id, 0),
        delta: some_f64_ne(dnc.delta, 0.0),
        price: some_f64_ne(dnc.price, 0.0),
    }
}

fn encode_combo_legs(contract: &Contract, order: Option<&Order>) -> Vec<proto::ComboLeg> {
    if contract.combo_legs.is_empty() {
        return Vec::new();
    }
    contract
        .combo_legs
        .iter()
        .enumerate()
        .map(|(i, leg)| {
            let per_leg_price = order.and_then(|o| o.order_combo_legs.get(i)).and_then(|ocl| ocl.price);
            encode_combo_leg(leg, per_leg_price)
        })
        .collect()
}

fn encode_combo_leg(leg: &contracts::ComboLeg, per_leg_price: Option<f64>) -> proto::ComboLeg {
    proto::ComboLeg {
        con_id: some_i32_ne(leg.contract_id, 0),
        ratio: some_i32_ne(leg.ratio, 0),
        action: Some(leg.action.to_string()),
        exchange: some_str(&leg.exchange),
        open_close: some_i32_ne(leg.open_close as i32, 0),
        short_sales_slot: some_i32_ne(leg.short_sale_slot, 0),
        designated_location: some_str(&leg.designated_location),
        exempt_code: some_i32_ne(leg.exempt_code, 0),
        per_leg_price,
    }
}

// === Scanner Subscription ===

pub fn encode_scanner_subscription(
    subscription: &crate::scanner::ScannerSubscription,
    filter: &[crate::contracts::TagValue],
) -> proto::ScannerSubscription {
    proto::ScannerSubscription {
        number_of_rows: some_i32_ne(subscription.number_of_rows, i32::MAX),
        instrument: subscription.instrument.clone(),
        location_code: subscription.location_code.clone(),
        scan_code: subscription.scan_code.clone(),
        above_price: subscription.above_price,
        below_price: subscription.below_price,
        above_volume: subscription.above_volume.map(|v| v as i64),
        market_cap_above: subscription.market_cap_above,
        market_cap_below: subscription.market_cap_below,
        moody_rating_above: subscription.moody_rating_above.clone(),
        moody_rating_below: subscription.moody_rating_below.clone(),
        sp_rating_above: subscription.sp_rating_above.clone(),
        sp_rating_below: subscription.sp_rating_below.clone(),
        maturity_date_above: subscription.maturity_date_above.clone(),
        maturity_date_below: subscription.maturity_date_below.clone(),
        coupon_rate_above: subscription.coupon_rate_above,
        coupon_rate_below: subscription.coupon_rate_below,
        exclude_convertible: some_bool(subscription.exclude_convertible),
        average_option_volume_above: subscription.average_option_volume_above.map(|v| v as i64),
        scanner_setting_pairs: subscription.scanner_setting_pairs.clone(),
        stock_type_filter: subscription.stock_type_filter.clone(),
        scanner_subscription_filter_options: tag_values_to_map(filter),
        scanner_subscription_options: Default::default(),
    }
}

// === Utilities ===

pub fn tag_values_to_map(tags: &[crate::contracts::TagValue]) -> std::collections::HashMap<String, String> {
    tags.iter().map(|tv| (tv.tag.clone(), tv.value.clone())).collect()
}

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;
