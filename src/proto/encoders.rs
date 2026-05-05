//! Shared domain→proto converters for outbound protobuf encoding.

use crate::contracts::{self, Contract};
use crate::orders::{self, Order, OrderCondition, SoftDollarTier};
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

pub(crate) fn some_bool(v: bool) -> Option<bool> {
    if v {
        Some(true)
    } else {
        None
    }
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
        right: some_str(&contract.right),
        multiplier: contract.multiplier.parse::<f64>().ok(),
        exchange: some_str(&contract.exchange.to_string()),
        primary_exch: some_str(&contract.primary_exchange.to_string()),
        currency: some_str(&contract.currency.to_string()),
        local_symbol: some_str(&contract.local_symbol),
        trading_class: some_str(&contract.trading_class),
        sec_id_type: some_str(&contract.security_id_type),
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
        action: some_str(&leg.action),
        exchange: some_str(&leg.exchange),
        open_close: some_i32_ne(leg.open_close as i32, 0),
        short_sales_slot: some_i32_ne(leg.short_sale_slot, 0),
        designated_location: some_str(&leg.designated_location),
        exempt_code: some_i32_ne(leg.exempt_code, 0),
        per_leg_price,
    }
}

// === Order ===

pub fn encode_order(order: &Order) -> proto::Order {
    let proto_order = proto::Order {
        client_id: some_i32_ne(order.client_id, 0),
        perm_id: some_i64_ne(order.perm_id, 0),
        parent_id: some_i32_ne(order.parent_id, 0),
        action: some_str(&order.action.to_string()),
        total_quantity: if order.total_quantity == 0.0 {
            None
        } else {
            Some(order.total_quantity.to_string())
        },
        display_size: order.display_size,
        order_type: some_str(&order.order_type),
        lmt_price: order.limit_price,
        aux_price: order.aux_price,
        tif: some_str(&order.tif.to_string()),
        account: some_str(&order.account),
        settling_firm: some_str(&order.settling_firm),
        clearing_account: some_str(&order.clearing_account),
        clearing_intent: some_str(&order.clearing_intent),
        all_or_none: some_bool(order.all_or_none),
        block_order: some_bool(order.block_order),
        hidden: some_bool(order.hidden),
        outside_rth: some_bool(order.outside_rth),
        sweep_to_fill: some_bool(order.sweep_to_fill),
        percent_offset: order.percent_offset,
        trailing_percent: order.trailing_percent,
        trail_stop_price: order.trail_stop_price,
        min_qty: order.min_qty,
        good_after_time: some_str(&order.good_after_time),
        good_till_date: some_str(&order.good_till_date),
        oca_group: some_str(&order.oca_group),
        order_ref: some_str(&order.order_ref),
        rule80_a: order.rule_80_a.as_ref().map(|r| r.to_string()),
        oca_type: some_i32_ne(i32::from(order.oca_type), 0),
        trigger_method: some_i32_ne(i32::from(order.trigger_method), 0),
        active_start_time: some_str(&order.active_start_time),
        active_stop_time: some_str(&order.active_stop_time),
        fa_group: some_str(&order.fa_group),
        fa_method: some_str(&order.fa_method),
        fa_percentage: some_str(&order.fa_percentage),
        volatility: order.volatility,
        volatility_type: order.volatility_type.map(|v| i32::from(v)),
        continuous_update: some_bool(order.continuous_update),
        reference_price_type: order.reference_price_type.map(|r| i32::from(r)),
        delta_neutral_order_type: some_str(&order.delta_neutral_order_type),
        delta_neutral_aux_price: order.delta_neutral_aux_price,
        delta_neutral_con_id: some_i32_ne(order.delta_neutral_con_id, 0),
        delta_neutral_open_close: some_str(&order.delta_neutral_open_close),
        delta_neutral_short_sale: some_bool(order.delta_neutral_short_sale),
        delta_neutral_short_sale_slot: some_i32_ne(order.delta_neutral_short_sale_slot, 0),
        delta_neutral_designated_location: some_str(&order.delta_neutral_designated_location),
        scale_init_level_size: order.scale_init_level_size,
        scale_subs_level_size: order.scale_subs_level_size,
        scale_price_increment: order.scale_price_increment,
        scale_price_adjust_value: order.scale_price_adjust_value,
        scale_price_adjust_interval: order.scale_price_adjust_interval,
        scale_profit_offset: order.scale_profit_offset,
        scale_auto_reset: some_bool(order.scale_auto_reset),
        scale_init_position: order.scale_init_position,
        scale_init_fill_qty: order.scale_init_fill_qty,
        scale_random_percent: some_bool(order.scale_random_percent),
        scale_table: some_str(&order.scale_table),
        hedge_type: some_str(&order.hedge_type),
        hedge_param: some_str(&order.hedge_param),
        algo_strategy: some_str(&order.algo_strategy),
        algo_params: tag_values_to_map(&order.algo_params),
        algo_id: some_str(&order.algo_id),
        smart_combo_routing_params: tag_values_to_map(&order.smart_combo_routing_params),
        what_if: some_bool(order.what_if),
        transmit: some_bool(order.transmit),
        override_percentage_constraints: some_bool(order.override_percentage_constraints),
        open_close: order.open_close.as_ref().map(|oc| oc.to_string()),
        origin: some_i32_ne(i32::from(order.origin), 0),
        short_sale_slot: some_i32_ne(i32::from(order.short_sale_slot), 0),
        designated_location: some_str(&order.designated_location),
        exempt_code: some_i32_ne(order.exempt_code, 0),
        delta_neutral_settling_firm: some_str(&order.delta_neutral_settling_firm),
        delta_neutral_clearing_account: some_str(&order.delta_neutral_clearing_account),
        delta_neutral_clearing_intent: some_str(&order.delta_neutral_clearing_intent),
        discretionary_amt: some_f64_ne(order.discretionary_amt, 0.0),
        opt_out_smart_routing: some_bool(order.opt_out_smart_routing),
        starting_price: order.starting_price,
        stock_ref_price: order.stock_ref_price,
        delta: order.delta,
        stock_range_lower: order.stock_range_lower,
        stock_range_upper: order.stock_range_upper,
        not_held: some_bool(order.not_held),
        order_misc_options: tag_values_to_map(&order.order_misc_options),
        solicited: some_bool(order.solicited),
        randomize_size: some_bool(order.randomize_size),
        randomize_price: some_bool(order.randomize_price),
        reference_contract_id: some_i32_ne(order.reference_contract_id, 0),
        pegged_change_amount: order.pegged_change_amount,
        is_pegged_change_amount_decrease: some_bool(order.is_pegged_change_amount_decrease),
        reference_change_amount: order.reference_change_amount,
        reference_exchange_id: some_str(&order.reference_exchange),
        adjusted_order_type: some_str(&order.adjusted_order_type),
        trigger_price: order.trigger_price,
        adjusted_stop_price: order.adjusted_stop_price,
        adjusted_stop_limit_price: order.adjusted_stop_limit_price,
        adjusted_trailing_amount: order.adjusted_trailing_amount,
        adjustable_trailing_unit: some_i32_ne(order.adjustable_trailing_unit, 0),
        lmt_price_offset: order.limit_price_offset,
        conditions: encode_conditions(&order.conditions),
        conditions_cancel_order: some_bool(order.conditions_cancel_order),
        conditions_ignore_rth: some_bool(order.conditions_ignore_rth),
        model_code: some_str(&order.model_code),
        ext_operator: some_str(&order.ext_operator),
        soft_dollar_tier: encode_soft_dollar_tier(&order.soft_dollar_tier),
        cash_qty: order.cash_qty,
        mifid2_decision_maker: some_str(&order.mifid2_decision_maker),
        mifid2_decision_algo: some_str(&order.mifid2_decision_algo),
        mifid2_execution_trader: some_str(&order.mifid2_execution_trader),
        mifid2_execution_algo: some_str(&order.mifid2_execution_algo),
        dont_use_auto_price_for_hedge: some_bool(order.dont_use_auto_price_for_hedge),
        is_oms_container: some_bool(order.is_oms_container),
        discretionary_up_to_limit_price: some_bool(order.discretionary_up_to_limit_price),
        use_price_mgmt_algo: if order.use_price_mgmt_algo { Some(1) } else { None },
        duration: order.duration,
        post_to_ats: order.post_to_ats,
        advanced_error_override: some_str(&order.advanced_error_override),
        manual_order_time: some_str(&order.manual_order_time),
        min_trade_qty: order.min_trade_qty,
        min_compete_size: order.min_compete_size,
        compete_against_best_offset: order.compete_against_best_offset,
        mid_offset_at_whole: order.mid_offset_at_whole,
        mid_offset_at_half: order.mid_offset_at_half,
        customer_account: some_str(&order.customer_account),
        professional_customer: some_bool(order.professional_customer),
        bond_accrued_interest: some_str(&order.bond_accrued_interest),
        include_overnight: some_bool(order.include_overnight),
        manual_order_indicator: order.manual_order_indicator,
        submitter: some_str(&order.submitter),
        auto_cancel_parent: some_bool(order.auto_cancel_parent),
        imbalance_only: some_bool(order.imbalance_only),
        // fields not directly mapped from our Order struct
        order_id: None,
        auto_cancel_date: some_str(&order.auto_cancel_date),
        filled_quantity: if order.filled_quantity == 0.0 {
            None
        } else {
            Some(order.filled_quantity.to_string())
        },
        ref_futures_con_id: order.ref_futures_con_id,
        shareholder: some_str(&order.shareholder),
        route_marketable_to_bbo: if order.route_marketable_to_bbo { Some(1) } else { None },
        parent_perm_id: order.parent_perm_id,
        deactivate: None,
        post_only: None,
        allow_pre_open: None,
        ignore_open_auction: None,
        seek_price_improvement: None,
        what_if_type: None,
    };

    proto_order
}

fn some_i64_ne(v: i64, default: i64) -> Option<i64> {
    if v == default {
        None
    } else {
        Some(v)
    }
}

fn encode_conditions(conditions: &[OrderCondition]) -> Vec<proto::OrderCondition> {
    conditions.iter().map(encode_condition).collect()
}

fn encode_condition(condition: &OrderCondition) -> proto::OrderCondition {
    let mut proto_cond = proto::OrderCondition {
        r#type: Some(condition.condition_type()),
        is_conjunction_connection: Some(condition.is_conjunction()),
        ..Default::default()
    };

    match condition {
        OrderCondition::Price(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.price = some_f64_ne(c.price, 0.0);
            // C# PriceCondition.Serialize always writes trigger_method (incl. Default=0);
            // omitting it makes TWS reject with "Invalid value in field # 6127".
            proto_cond.trigger_method = Some(i32::from(c.trigger_method));
        }
        OrderCondition::Time(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.time = some_str(&c.time);
        }
        OrderCondition::Margin(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.percent = some_i32_ne(c.percent, 0);
        }
        OrderCondition::Execution(c) => {
            proto_cond.symbol = some_str(&c.symbol);
            proto_cond.sec_type = some_str(&c.security_type);
            proto_cond.exchange = some_str(&c.exchange);
        }
        OrderCondition::Volume(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.volume = some_i32_ne(c.volume, 0);
        }
        OrderCondition::PercentChange(c) => {
            proto_cond.is_more = Some(c.is_more);
            proto_cond.con_id = some_i32_ne(c.contract_id, 0);
            proto_cond.exchange = some_str(&c.exchange);
            proto_cond.change_percent = some_f64_ne(c.percent, 0.0);
        }
    }

    proto_cond
}

fn encode_soft_dollar_tier(tier: &SoftDollarTier) -> Option<proto::SoftDollarTier> {
    if tier.name.is_empty() && tier.value.is_empty() && tier.display_name.is_empty() {
        return None;
    }
    Some(proto::SoftDollarTier {
        name: some_str(&tier.name),
        value: some_str(&tier.value),
        display_name: some_str(&tier.display_name),
    })
}

// === Execution Filter ===

pub fn encode_execution_filter(filter: &orders::ExecutionFilter) -> proto::ExecutionFilter {
    proto::ExecutionFilter {
        client_id: filter.client_id,
        acct_code: some_str(&filter.account_code),
        time: some_str(&filter.time),
        symbol: some_str(&filter.symbol),
        sec_type: some_str(&filter.security_type),
        exchange: some_str(&filter.exchange),
        side: some_str(&filter.side),
        last_n_days: some_i32_ne(filter.last_n_days, 0),
        specific_dates: filter.specific_dates.iter().filter_map(|d| d.parse::<i32>().ok()).collect(),
    }
}

// === Scanner Subscription ===

pub fn encode_scanner_subscription(
    subscription: &crate::scanner::ScannerSubscription,
    filter: &[crate::orders::TagValue],
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

// === OrderCancel ===

pub fn encode_order_cancel(manual_order_cancel_time: &str) -> proto::OrderCancel {
    proto::OrderCancel {
        manual_order_cancel_time: some_str(manual_order_cancel_time),
        ext_operator: None,
        manual_order_indicator: None,
    }
}

// === Utilities ===

pub fn tag_values_to_map(tags: &[crate::orders::TagValue]) -> std::collections::HashMap<String, String> {
    tags.iter().map(|tv| (tv.tag.clone(), tv.value.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
    use crate::orders::{Action, Order, TimeInForce};

    #[test]
    fn test_encode_contract_basic() {
        let contract = Contract {
            contract_id: 265598,
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            primary_exchange: Exchange::from("NASDAQ"),
            currency: Currency::from("USD"),
            ..Default::default()
        };

        let proto = encode_contract(&contract);

        assert_eq!(proto.con_id, Some(265598));
        assert_eq!(proto.symbol.as_deref(), Some("AAPL"));
        assert_eq!(proto.sec_type.as_deref(), Some("STK"));
        assert_eq!(proto.exchange.as_deref(), Some("SMART"));
        assert_eq!(proto.primary_exch.as_deref(), Some("NASDAQ"));
        assert_eq!(proto.currency.as_deref(), Some("USD"));
        assert!(proto.last_trade_date_or_contract_month.is_none());
        assert!(proto.strike.is_none());
        assert!(proto.multiplier.is_none());
    }

    #[test]
    fn test_encode_contract_with_multiplier() {
        let contract = Contract {
            multiplier: "100".to_string(),
            ..Default::default()
        };

        let proto = encode_contract(&contract);
        assert_eq!(proto.multiplier, Some(100.0));
    }

    #[test]
    fn test_encode_contract_empty_multiplier() {
        let contract = Contract::default();
        let proto = encode_contract(&contract);
        assert!(proto.multiplier.is_none());
    }

    #[test]
    fn test_encode_delta_neutral() {
        let dnc = contracts::DeltaNeutralContract {
            contract_id: 123,
            delta: 0.5,
            price: 45.0,
        };
        let contract = Contract {
            delta_neutral_contract: Some(dnc),
            ..Default::default()
        };

        let proto = encode_contract(&contract);
        let dnc_proto = proto.delta_neutral_contract.unwrap();
        assert_eq!(dnc_proto.con_id, Some(123));
        assert_eq!(dnc_proto.delta, Some(0.5));
        assert_eq!(dnc_proto.price, Some(45.0));
    }

    #[test]
    fn test_encode_order_basic() {
        let order = Order {
            action: Action::Buy,
            total_quantity: 100.0,
            order_type: "LMT".to_string(),
            limit_price: Some(150.0),
            tif: TimeInForce::Day,
            transmit: true,
            ..Default::default()
        };

        let proto = encode_order(&order);

        assert_eq!(proto.action.as_deref(), Some("BUY"));
        assert_eq!(proto.total_quantity.as_deref(), Some("100"));
        assert_eq!(proto.order_type.as_deref(), Some("LMT"));
        assert_eq!(proto.lmt_price, Some(150.0));
        assert_eq!(proto.tif.as_deref(), Some("DAY"));
        assert_eq!(proto.transmit, Some(true));
    }

    #[test]
    fn test_encode_order_default_fields_omitted() {
        let order = Order::default();
        let proto = encode_order(&order);

        assert!(proto.client_id.is_none());
        assert!(proto.parent_id.is_none());
        assert!(proto.block_order.is_none());
        assert!(proto.hidden.is_none());
        assert!(proto.all_or_none.is_none());
    }

    #[test]
    fn test_encode_soft_dollar_tier_empty() {
        let tier = SoftDollarTier::default();
        assert!(encode_soft_dollar_tier(&tier).is_none());
    }

    #[test]
    fn test_encode_soft_dollar_tier_filled() {
        let tier = SoftDollarTier {
            name: "Tier1".to_string(),
            value: "Val1".to_string(),
            display_name: "Display1".to_string(),
        };
        let proto = encode_soft_dollar_tier(&tier).unwrap();
        assert_eq!(proto.name.as_deref(), Some("Tier1"));
        assert_eq!(proto.value.as_deref(), Some("Val1"));
        assert_eq!(proto.display_name.as_deref(), Some("Display1"));
    }

    #[test]
    fn test_encode_condition_price() {
        use crate::orders::conditions::{PriceCondition, TriggerMethod};
        let cond = OrderCondition::Price(PriceCondition {
            contract_id: 265598,
            exchange: "SMART".to_string(),
            price: 150.0,
            trigger_method: TriggerMethod::Last,
            is_more: true,
            is_conjunction: true,
        });

        let proto = encode_condition(&cond);
        assert_eq!(proto.r#type, Some(1));
        assert_eq!(proto.is_conjunction_connection, Some(true));
        assert_eq!(proto.is_more, Some(true));
        assert_eq!(proto.con_id, Some(265598));
        assert_eq!(proto.exchange.as_deref(), Some("SMART"));
        assert_eq!(proto.price, Some(150.0));
        assert_eq!(proto.trigger_method, Some(2)); // Last = 2
    }

    #[test]
    fn test_encode_condition_price_default_trigger_method_is_emitted() {
        // TWS rejects ("Invalid value in field # 6127") if trigger_method is omitted.
        use crate::orders::conditions::{PriceCondition, TriggerMethod};
        let cond = OrderCondition::Price(PriceCondition {
            contract_id: 265598,
            exchange: "SMART".to_string(),
            price: 150.0,
            trigger_method: TriggerMethod::Default,
            is_more: true,
            is_conjunction: true,
        });

        let proto = encode_condition(&cond);
        assert_eq!(proto.trigger_method, Some(0), "Default trigger_method must be emitted, not omitted");
    }

    #[test]
    fn test_encode_condition_time() {
        use crate::orders::conditions::TimeCondition;
        let cond = OrderCondition::Time(TimeCondition {
            time: "20251230 14:30:00 US/Eastern".to_string(),
            is_more: false,
            is_conjunction: false,
        });

        let proto = encode_condition(&cond);
        assert_eq!(proto.r#type, Some(3));
        assert_eq!(proto.is_conjunction_connection, Some(false));
        assert_eq!(proto.is_more, Some(false));
        assert_eq!(proto.time.as_deref(), Some("20251230 14:30:00 US/Eastern"));
    }

    #[test]
    fn test_encode_execution_filter() {
        let filter = orders::ExecutionFilter {
            client_id: Some(1),
            account_code: "DU123".to_string(),
            time: "20240101 00:00:00".to_string(),
            symbol: "AAPL".to_string(),
            security_type: "STK".to_string(),
            exchange: "SMART".to_string(),
            side: "BUY".to_string(),
            last_n_days: 5,
            specific_dates: vec!["20240101".to_string()],
        };

        let proto = encode_execution_filter(&filter);
        assert_eq!(proto.client_id, Some(1));
        assert_eq!(proto.acct_code.as_deref(), Some("DU123"));
        assert_eq!(proto.symbol.as_deref(), Some("AAPL"));
        assert_eq!(proto.last_n_days, Some(5));
        assert_eq!(proto.specific_dates, vec![20240101]);
    }

    #[test]
    fn test_some_str_empty() {
        assert!(some_str("").is_none());
        assert_eq!(some_str("hello"), Some("hello".to_string()));
    }

    #[test]
    fn test_tag_values_to_map() {
        let tags = vec![
            crate::orders::TagValue {
                tag: "k1".to_string(),
                value: "v1".to_string(),
            },
            crate::orders::TagValue {
                tag: "k2".to_string(),
                value: "v2".to_string(),
            },
        ];
        let map = tag_values_to_map(&tags);
        assert_eq!(map.get("k1").unwrap(), "v1");
        assert_eq!(map.get("k2").unwrap(), "v2");
    }
}
