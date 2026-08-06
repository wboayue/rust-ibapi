use crate::contracts::{
    ComboLeg, ComboLegOpenClose, Contract, ContractDetails, Currency, DeltaNeutralContract, Exchange, FundAssetType, FundDistributionPolicyIndicator,
    IneligibilityReason, LegAction, SecurityIdType, SecurityType, Symbol, TagValue,
};
use crate::orders::conditions::TriggerMethod;
use crate::orders::{
    Action, Execution, ExecutionSide, Liquidity, OcaType, Order, OrderAllocation, OrderCondition, OrderOpenClose, OrderOrigin, OrderState,
    ReferencePriceType, Rule80A, ShortSaleSlot, SoftDollarTier, TimeInForce, VolatilityType,
};
use crate::proto;
use crate::Error;

// === Helper functions ===

pub(crate) fn s(opt: &Option<String>) -> String {
    opt.clone().unwrap_or_default()
}

pub(crate) fn parse_i32(opt: &Option<String>) -> i32 {
    opt.as_deref().and_then(|s| s.parse::<i32>().ok()).unwrap_or_default()
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
/// silently became `0` — data loss on fractional (crypto) sizes, issue #716. Per
/// CLAUDE.md rule 16, a decoder rejects malformed input rather than defaulting.
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
    // rejects inf/NaN, which f64::from_str accepts but no size can be.
    if !value.is_finite() || value == f64::MAX {
        return Ok(None);
    }
    Ok(Some(value))
}

/// [`parse_optional_decimal`], collapsing "no value" to `0.0`.
///
/// **Transitional.** It exists only for fields whose public type is still plain
/// `f64` and so cannot represent "unset". `0.0` is what those fields already
/// produced for an absent or empty wire value, so this preserves today's
/// behaviour while still erroring on malformed input.
///
/// Every call site is a candidate for `Option<f64>` in the follow-up
/// decimal-quantity work — grep this name for the worklist.
pub(crate) fn parse_decimal_or_zero(opt: Option<&str>) -> Result<f64, Error> {
    Ok(parse_optional_decimal(opt)?.unwrap_or(0.0))
}

/// Parse a required enumerated wire field. Both `None` and empty strings
/// surface as `Error::Parse`: an empty wire on a required field is a TWS
/// protocol bug, not a default. Silent fallback to `T::default()` masks
/// incomplete responses (CLAUDE.md rule 16).
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
        security_type: SecurityType::from(proto.sec_type.as_deref().unwrap_or_default()),
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

pub fn decode_order(proto: &proto::Order) -> Result<Order, Error> {
    let mut order = Order::default();

    order.client_id = proto.client_id.unwrap_or_default();
    order.order_id = proto.order_id.unwrap_or_default();
    order.perm_id = proto.perm_id.unwrap_or_default();
    order.parent_id = proto.parent_id.unwrap_or_default();

    order.action = Action::from(proto.action.as_deref().unwrap_or("BUY"));
    // pattern D: absent means 0 upstream (Order.cs leaves TotalQuantity at decimal's default)
    order.total_quantity = parse_decimal_or_zero(proto.total_quantity.as_deref())?;
    order.display_size = proto.display_size.map(Some).unwrap_or(Some(0));
    order.order_type = s(&proto.order_type);
    order.limit_price = optional_f64(proto.lmt_price);
    order.aux_price = optional_f64(proto.aux_price);
    order.tif = TimeInForce::from(proto.tif.as_deref().unwrap_or("DAY"));

    // clearing info
    order.account = s(&proto.account);
    order.settling_firm = s(&proto.settling_firm);
    order.clearing_account = s(&proto.clearing_account);
    order.clearing_intent = s(&proto.clearing_intent);

    // secondary attributes
    order.all_or_none = proto.all_or_none.unwrap_or_default();
    order.block_order = proto.block_order.unwrap_or_default();
    order.hidden = proto.hidden.unwrap_or_default();
    order.outside_rth = proto.outside_rth.unwrap_or_default();
    order.sweep_to_fill = proto.sweep_to_fill.unwrap_or_default();
    order.percent_offset = optional_f64(proto.percent_offset);
    order.trailing_percent = optional_f64(proto.trailing_percent);
    order.trail_stop_price = optional_f64(proto.trail_stop_price);
    order.min_qty = proto.min_qty;
    order.good_after_time = s(&proto.good_after_time);
    order.good_till_date = s(&proto.good_till_date);
    order.oca_group = s(&proto.oca_group);
    order.order_ref = s(&proto.order_ref);
    order.rule_80_a = proto.rule80_a.as_deref().and_then(Rule80A::from);
    order.oca_type = OcaType::from(proto.oca_type.unwrap_or_default());
    order.trigger_method = TriggerMethod::from(proto.trigger_method.unwrap_or_default());

    // extended order fields
    order.active_start_time = s(&proto.active_start_time);
    order.active_stop_time = s(&proto.active_stop_time);

    // advisor allocation
    order.fa_group = s(&proto.fa_group);
    order.fa_method = s(&proto.fa_method);
    order.fa_percentage = s(&proto.fa_percentage);

    // volatility orders
    order.volatility = optional_f64(proto.volatility);
    order.volatility_type = proto.volatility_type.map(VolatilityType::from);
    order.continuous_update = proto.continuous_update.unwrap_or_default();
    order.reference_price_type = proto.reference_price_type.map(ReferencePriceType::from);
    order.delta_neutral_order_type = s(&proto.delta_neutral_order_type);
    order.delta_neutral_aux_price = optional_f64(proto.delta_neutral_aux_price);
    order.delta_neutral_con_id = proto.delta_neutral_con_id.unwrap_or_default();
    order.delta_neutral_open_close = s(&proto.delta_neutral_open_close);
    order.delta_neutral_short_sale = proto.delta_neutral_short_sale.unwrap_or_default();
    order.delta_neutral_short_sale_slot = proto.delta_neutral_short_sale_slot.unwrap_or_default();
    order.delta_neutral_designated_location = s(&proto.delta_neutral_designated_location);

    // scale orders
    order.scale_init_level_size = proto.scale_init_level_size;
    order.scale_subs_level_size = proto.scale_subs_level_size;
    order.scale_price_increment = optional_f64(proto.scale_price_increment);
    order.scale_price_adjust_value = optional_f64(proto.scale_price_adjust_value);
    order.scale_price_adjust_interval = proto.scale_price_adjust_interval;
    order.scale_profit_offset = optional_f64(proto.scale_profit_offset);
    order.scale_auto_reset = proto.scale_auto_reset.unwrap_or_default();
    order.scale_init_position = proto.scale_init_position;
    order.scale_init_fill_qty = proto.scale_init_fill_qty;
    order.scale_random_percent = proto.scale_random_percent.unwrap_or_default();
    order.scale_table = s(&proto.scale_table);

    // hedge orders
    order.hedge_type = s(&proto.hedge_type);
    order.hedge_param = s(&proto.hedge_param);
    order.hedge_max_size = proto.hedge_max_size;

    // algo orders
    order.algo_strategy = s(&proto.algo_strategy);
    order.algo_params = tag_values(&proto.algo_params);
    order.algo_id = s(&proto.algo_id);

    // combo orders
    order.smart_combo_routing_params = tag_values(&proto.smart_combo_routing_params);

    // processing control
    order.what_if = proto.what_if.unwrap_or_default();
    order.transmit = proto.transmit.unwrap_or(true);
    order.deactivate = proto.deactivate.unwrap_or_default();
    order.override_percentage_constraints = proto.override_percentage_constraints.unwrap_or_default();

    // institutional orders
    order.open_close = proto.open_close.as_deref().and_then(OrderOpenClose::from);
    order.origin = OrderOrigin::from(proto.origin.unwrap_or_default());
    order.short_sale_slot = ShortSaleSlot::from(proto.short_sale_slot.unwrap_or_default());
    order.designated_location = s(&proto.designated_location);
    order.exempt_code = proto.exempt_code.unwrap_or(-1);
    order.delta_neutral_settling_firm = s(&proto.delta_neutral_settling_firm);
    order.delta_neutral_clearing_account = s(&proto.delta_neutral_clearing_account);
    order.delta_neutral_clearing_intent = s(&proto.delta_neutral_clearing_intent);

    // SMART routing
    order.discretionary_amt = proto.discretionary_amt.unwrap_or_default();
    order.opt_out_smart_routing = proto.opt_out_smart_routing.unwrap_or_default();

    // BOX orders
    order.starting_price = optional_f64(proto.starting_price);
    order.stock_ref_price = optional_f64(proto.stock_ref_price);
    order.delta = optional_f64(proto.delta);

    // pegged orders
    order.stock_range_lower = optional_f64(proto.stock_range_lower);
    order.stock_range_upper = optional_f64(proto.stock_range_upper);

    // not held
    order.not_held = proto.not_held.unwrap_or_default();

    // order misc options
    order.order_misc_options = tag_values(&proto.order_misc_options);

    // solicited / randomize
    order.solicited = proto.solicited.unwrap_or_default();
    order.randomize_size = proto.randomize_size.unwrap_or_default();
    order.randomize_price = proto.randomize_price.unwrap_or_default();

    // PEG2BENCH fields
    order.reference_contract_id = proto.reference_contract_id.unwrap_or_default();
    order.pegged_change_amount = optional_f64(proto.pegged_change_amount);
    order.is_pegged_change_amount_decrease = proto.is_pegged_change_amount_decrease.unwrap_or_default();
    order.reference_change_amount = optional_f64(proto.reference_change_amount);
    order.reference_exchange = s(&proto.reference_exchange_id);
    order.adjusted_order_type = s(&proto.adjusted_order_type);
    order.trigger_price = optional_f64(proto.trigger_price);
    order.adjusted_stop_price = optional_f64(proto.adjusted_stop_price);
    order.adjusted_stop_limit_price = optional_f64(proto.adjusted_stop_limit_price);
    order.adjusted_trailing_amount = optional_f64(proto.adjusted_trailing_amount);
    order.adjustable_trailing_unit = proto.adjustable_trailing_unit.unwrap_or_default();
    order.limit_price_offset = optional_f64(proto.lmt_price_offset);

    // conditions
    order.conditions = proto.conditions.iter().map(decode_order_condition).collect();
    order.conditions_cancel_order = proto.conditions_cancel_order.unwrap_or_default();
    order.conditions_ignore_rth = proto.conditions_ignore_rth.unwrap_or_default();

    // models
    order.model_code = s(&proto.model_code);
    order.ext_operator = s(&proto.ext_operator);
    order.soft_dollar_tier = proto.soft_dollar_tier.as_ref().map(decode_soft_dollar_tier).unwrap_or_default();

    // native cash quantity
    order.cash_qty = optional_f64(proto.cash_qty);

    // MIFID2
    order.mifid2_decision_maker = s(&proto.mifid2_decision_maker);
    order.mifid2_decision_algo = s(&proto.mifid2_decision_algo);
    order.mifid2_execution_trader = s(&proto.mifid2_execution_trader);
    order.mifid2_execution_algo = s(&proto.mifid2_execution_algo);

    // additional fields
    order.dont_use_auto_price_for_hedge = proto.dont_use_auto_price_for_hedge.unwrap_or_default();
    order.is_oms_container = proto.is_oms_container.unwrap_or_default();
    order.discretionary_up_to_limit_price = proto.discretionary_up_to_limit_price.unwrap_or_default();
    order.auto_cancel_date = s(&proto.auto_cancel_date);
    // pattern U: absent means unset upstream (Order.cs ctor sets decimal.MaxValue)
    order.filled_quantity = parse_decimal_or_zero(proto.filled_quantity.as_deref())?;
    order.ref_futures_con_id = proto.ref_futures_con_id.map(Some).unwrap_or(Some(0));
    order.auto_cancel_parent = proto.auto_cancel_parent.unwrap_or_default();
    order.shareholder = s(&proto.shareholder);
    order.imbalance_only = proto.imbalance_only.unwrap_or_default();
    order.route_marketable_to_bbo = proto.route_marketable_to_bbo.unwrap_or_default() != 0;
    order.parent_perm_id = proto.parent_perm_id;
    order.use_price_mgmt_algo = proto.use_price_mgmt_algo.unwrap_or_default() != 0;
    order.duration = proto.duration;
    order.post_to_ats = proto.post_to_ats;
    order.advanced_error_override = s(&proto.advanced_error_override);
    order.manual_order_time = s(&proto.manual_order_time);
    order.min_trade_qty = proto.min_trade_qty;
    order.min_compete_size = proto.min_compete_size;
    order.compete_against_best_offset = optional_f64(proto.compete_against_best_offset);
    order.mid_offset_at_whole = optional_f64(proto.mid_offset_at_whole);
    order.mid_offset_at_half = optional_f64(proto.mid_offset_at_half);
    order.customer_account = s(&proto.customer_account);
    order.professional_customer = proto.professional_customer.unwrap_or_default();
    order.bond_accrued_interest = s(&proto.bond_accrued_interest);
    order.include_overnight = proto.include_overnight.unwrap_or_default();
    order.manual_order_indicator = proto.manual_order_indicator;
    order.submitter = s(&proto.submitter);

    Ok(order)
}

fn decode_order_condition(proto: &proto::OrderCondition) -> OrderCondition {
    use crate::orders::conditions::*;

    let condition_type = proto.r#type.unwrap_or_default();
    let is_conjunction = proto.is_conjunction_connection.unwrap_or(true);
    let is_more = proto.is_more.unwrap_or_default();

    match condition_type {
        1 => OrderCondition::Price(PriceCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            price: proto.price.unwrap_or_default(),
            is_more,
            is_conjunction,
            trigger_method: TriggerMethod::from(proto.trigger_method.unwrap_or_default()),
        }),
        3 => OrderCondition::Time(TimeCondition {
            time: s(&proto.time),
            is_more,
            is_conjunction,
        }),
        4 => OrderCondition::Margin(MarginCondition {
            percent: proto.percent.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        5 => OrderCondition::Execution(ExecutionCondition {
            symbol: s(&proto.symbol),
            security_type: s(&proto.sec_type),
            exchange: s(&proto.exchange),
            is_conjunction,
        }),
        6 => OrderCondition::Volume(VolumeCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            volume: proto.volume.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        7 => OrderCondition::PercentChange(PercentChangeCondition {
            contract_id: proto.con_id.unwrap_or_default(),
            exchange: s(&proto.exchange),
            percent: proto.change_percent.unwrap_or_default(),
            is_more,
            is_conjunction,
        }),
        _ => OrderCondition::Price(PriceCondition::default()),
    }
}

pub fn decode_order_state(proto: &proto::OrderState) -> Result<OrderState, Error> {
    Ok(OrderState {
        status: parse_required(proto.status.as_deref(), "OrderStatus")?,
        initial_margin_before: optional_f64(proto.init_margin_before),
        maintenance_margin_before: optional_f64(proto.maint_margin_before),
        equity_with_loan_before: optional_f64(proto.equity_with_loan_before),
        initial_margin_change: optional_f64(proto.init_margin_change),
        maintenance_margin_change: optional_f64(proto.maint_margin_change),
        equity_with_loan_change: optional_f64(proto.equity_with_loan_change),
        initial_margin_after: optional_f64(proto.init_margin_after),
        maintenance_margin_after: optional_f64(proto.maint_margin_after),
        equity_with_loan_after: optional_f64(proto.equity_with_loan_after),
        commission: optional_f64(proto.commission_and_fees),
        minimum_commission: optional_f64(proto.min_commission_and_fees),
        maximum_commission: optional_f64(proto.max_commission_and_fees),
        commission_currency: s(&proto.commission_and_fees_currency),
        margin_currency: s(&proto.margin_currency),
        initial_margin_before_outside_rth: optional_f64(proto.init_margin_before_outside_rth),
        maintenance_margin_before_outside_rth: optional_f64(proto.maint_margin_before_outside_rth),
        equity_with_loan_before_outside_rth: optional_f64(proto.equity_with_loan_before_outside_rth),
        initial_margin_change_outside_rth: optional_f64(proto.init_margin_change_outside_rth),
        maintenance_margin_change_outside_rth: optional_f64(proto.maint_margin_change_outside_rth),
        equity_with_loan_change_outside_rth: optional_f64(proto.equity_with_loan_change_outside_rth),
        initial_margin_after_outside_rth: optional_f64(proto.init_margin_after_outside_rth),
        maintenance_margin_after_outside_rth: optional_f64(proto.maint_margin_after_outside_rth),
        equity_with_loan_after_outside_rth: optional_f64(proto.equity_with_loan_after_outside_rth),
        suggested_size: parse_optional_decimal(proto.suggested_size.as_deref())?,
        reject_reason: s(&proto.reject_reason),
        order_allocations: proto
            .order_allocations
            .iter()
            .map(decode_order_allocation)
            .collect::<Result<Vec<_>, Error>>()?,
        warning_text: s(&proto.warning_text),
        completed_time: s(&proto.completed_time),
        completed_status: s(&proto.completed_status),
    })
}

fn decode_order_allocation(proto: &proto::OrderAllocation) -> Result<OrderAllocation, Error> {
    Ok(OrderAllocation {
        account: s(&proto.account),
        position: parse_optional_decimal(proto.position.as_deref())?,
        position_desired: parse_optional_decimal(proto.position_desired.as_deref())?,
        position_after: parse_optional_decimal(proto.position_after.as_deref())?,
        desired_alloc_qty: parse_optional_decimal(proto.desired_alloc_qty.as_deref())?,
        allowed_alloc_qty: parse_optional_decimal(proto.allowed_alloc_qty.as_deref())?,
        is_monetary: proto.is_monetary.unwrap_or_default(),
    })
}

pub fn decode_execution(proto: &proto::Execution) -> Result<Execution, Error> {
    Ok(Execution {
        order_id: proto.order_id.unwrap_or_default(),
        client_id: proto.client_id.unwrap_or_default(),
        execution_id: s(&proto.exec_id),
        time: s(&proto.time),
        account_number: s(&proto.acct_number),
        exchange: s(&proto.exchange),
        side: parse_required::<ExecutionSide>(proto.side.as_deref(), "Execution.side")?,
        // pattern D: absent means 0 upstream (Execution.cs ctor sets Shares/CumQty to 0)
        shares: parse_decimal_or_zero(proto.shares.as_deref())?,
        price: proto.price.unwrap_or_default(),
        perm_id: proto.perm_id.unwrap_or_default(),
        liquidation: if proto.is_liquidation.unwrap_or_default() { 1 } else { 0 },
        // pattern D, as above
        cumulative_quantity: parse_decimal_or_zero(proto.cum_qty.as_deref())?,
        average_price: proto.avg_price.unwrap_or_default(),
        order_reference: s(&proto.order_ref),
        ev_rule: s(&proto.ev_rule),
        ev_multiplier: optional_f64(proto.ev_multiplier),
        model_code: s(&proto.model_code),
        last_liquidity: Liquidity::from(proto.last_liquidity.unwrap_or_default()),
        pending_price_revision: proto.is_price_revision_pending.unwrap_or_default(),
        submitter: s(&proto.submitter),
    })
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
        // pattern U: absent means unset upstream (ContractDetails.cs ctor sets
        // decimal.MaxValue). These three become Option<f64> in the follow-up PR.
        min_size: parse_decimal_or_zero(proto_details.min_size.as_deref())?,
        size_increment: parse_decimal_or_zero(proto_details.size_increment.as_deref())?,
        suggested_size_increment: parse_decimal_or_zero(proto_details.suggested_size_increment.as_deref())?,
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
