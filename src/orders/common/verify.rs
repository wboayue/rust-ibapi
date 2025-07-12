use crate::contracts::Contract;
use crate::orders::Order;
use crate::{server_versions, Client, Error};

// Verifies that Order is properly formed.
pub(crate) fn verify_order(client: &Client, order: &Order, _order_id: i32) -> Result<(), Error> {
    let is_bag_order: bool = false; // StringsAreEqual(Constants.BagSecType, contract.SecType)

    if order.scale_init_level_size.is_some() || order.scale_price_increment.is_some() {
        client.check_server_version(server_versions::SCALE_ORDERS, "It does not support Scale orders.")?
    }

    if order.what_if {
        client.check_server_version(server_versions::WHAT_IF_ORDERS, "It does not support what-if orders.")?
    }

    if order.scale_subs_level_size.is_some() {
        client.check_server_version(
            server_versions::SCALE_ORDERS2,
            "It does not support Subsequent Level Size for Scale orders.",
        )?
    }

    if !order.algo_strategy.is_empty() {
        client.check_server_version(server_versions::ALGO_ORDERS, "It does not support algo orders.")?
    }

    if order.not_held {
        client.check_server_version(server_versions::NOT_HELD, "It does not support not_held parameter.")?
    }

    if order.exempt_code != -1 {
        client.check_server_version(server_versions::SSHORTX, "It does not support exempt_code parameter.")?
    }

    if !order.hedge_type.is_empty() {
        client.check_server_version(server_versions::HEDGE_ORDERS, "It does not support hedge orders.")?
    }

    if order.opt_out_smart_routing {
        client.check_server_version(
            server_versions::OPT_OUT_SMART_ROUTING,
            "It does not support opt_out_smart_routing parameter.",
        )?
    }

    if order.delta_neutral_con_id > 0
        || !order.delta_neutral_settling_firm.is_empty()
        || !order.delta_neutral_clearing_account.is_empty()
        || !order.delta_neutral_clearing_intent.is_empty()
    {
        client.check_server_version(
            server_versions::DELTA_NEUTRAL_CONID,
            "It does not support delta_neutral parameters: con_id, settling_firm, clearing_account, clearing_intent.",
        )?
    }

    if !order.delta_neutral_open_close.is_empty()
        || order.delta_neutral_short_sale
        || order.delta_neutral_short_sale_slot > 0
        || !order.delta_neutral_designated_location.is_empty()
    {
        client.check_server_version(
            server_versions::DELTA_NEUTRAL_OPEN_CLOSE,
            "It does not support delta_neutral parameters: open_close, short_sale, short_saleSlot, designated_location",
        )?
    }

    if (order.scale_price_increment > Some(0.0))
        && (order.scale_price_adjust_value.is_some()
            || order.scale_price_adjust_interval.is_some()
            || order.scale_profit_offset.is_some()
            || order.scale_auto_reset
            || order.scale_init_position.is_some()
            || order.scale_init_fill_qty.is_some()
            || order.scale_random_percent)
    {
        client.check_server_version(
                server_versions::SCALE_ORDERS3,
                "It does not support Scale order parameters: PriceAdjustValue, PriceAdjustInterval, ProfitOffset, AutoReset, InitPosition, InitFillQty and RandomPercent",
            )?
    }

    if is_bag_order && order.order_combo_legs.iter().any(|combo_leg| combo_leg.price.is_some()) {
        client.check_server_version(
            server_versions::ORDER_COMBO_LEGS_PRICE,
            "It does not support per-leg prices for order combo legs.",
        )?
    }

    if order.trailing_percent.is_some() {
        client.check_server_version(server_versions::TRAILING_PERCENT, "It does not support trailing percent parameter.")?
    }

    if !order.algo_id.is_empty() {
        client.check_server_version(server_versions::ALGO_ID, "It does not support algo_id parameter")?
    }

    if !order.scale_table.is_empty() || !order.active_start_time.is_empty() || !order.active_stop_time.is_empty() {
        client.check_server_version(
            server_versions::SCALE_TABLE,
            "It does not support scale_table, active_start_time nor active_stop_time parameters.",
        )?
    }

    if !order.ext_operator.is_empty() {
        client.check_server_version(server_versions::EXT_OPERATOR, "It does not support ext_operator parameter")?
    }

    if order.cash_qty.is_some() {
        client.check_server_version(server_versions::CASH_QTY, "It does not support cash_qty parameter")?
    }

    if !order.mifid2_execution_trader.is_empty() || !order.mifid2_execution_algo.is_empty() {
        client.check_server_version(server_versions::DECISION_MAKER, "It does not support MIFID II execution parameters")?
    }

    if order.dont_use_auto_price_for_hedge {
        client.check_server_version(
            server_versions::AUTO_PRICE_FOR_HEDGE,
            "It does not support don't use auto price for hedge parameter",
        )?
    }

    if order.is_oms_container {
        client.check_server_version(server_versions::ORDER_CONTAINER, "It does not support oms container parameter")?
    }

    if order.discretionary_up_to_limit_price {
        client.check_server_version(server_versions::D_PEG_ORDERS, "It does not support D-Peg orders")?
    }

    if order.use_price_mgmt_algo {
        client.check_server_version(server_versions::PRICE_MGMT_ALGO, "It does not support Use Price Management Algo requests")?
    }

    if order.duration.is_some() {
        client.check_server_version(server_versions::DURATION, "It does not support duration attribute")?
    }

    if order.post_to_ats.is_some() {
        client.check_server_version(server_versions::POST_TO_ATS, "It does not support post_to_ats attribute")?
    }

    if order.auto_cancel_parent {
        client.check_server_version(server_versions::AUTO_CANCEL_PARENT, "It does not support auto_cancel_parent attribute")?
    }

    if !order.advanced_error_override.is_empty() {
        client.check_server_version(
            server_versions::ADVANCED_ORDER_REJECT,
            "It does not support advanced error override attribute",
        )?
    }

    if !order.manual_order_time.is_empty() {
        client.check_server_version(server_versions::MANUAL_ORDER_TIME, "It does not support manual order time attribute")?
    }

    if order.min_trade_qty.is_some()
        || order.min_compete_size.is_some()
        || order.compete_against_best_offset.is_some()
        || order.mid_offset_at_whole.is_some()
        || order.mid_offset_at_half.is_some()
    {
        client.check_server_version(
            server_versions::PEGBEST_PEGMID_OFFSETS,
            "It does not support PEG BEST / PEG MID order parameters: minTradeQty, minCompeteSize, competeAgainstBestOffset, midOffsetAtWhole and midOffsetAtHalf",
        )?
    }

    Ok(())
}

// Verifies that Contract is properly formed.
pub(crate) fn verify_order_contract(client: &Client, contract: &Contract, _order_id: i32) -> Result<(), Error> {
    if contract
        .combo_legs
        .iter()
        .any(|combo_leg| combo_leg.short_sale_slot != 0 || !combo_leg.designated_location.is_empty())
    {
        client.check_server_version(server_versions::SSHORT_COMBO_LEGS, "It does not support SSHORT flag for combo legs")?
    }

    if contract.delta_neutral_contract.is_some() {
        client.check_server_version(server_versions::DELTA_NEUTRAL, "It does not support delta-neutral orders")?
    }

    if contract.contract_id > 0 {
        client.check_server_version(server_versions::PLACE_ORDER_CONID, "It does not support contract_id parameter")?
    }

    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(server_versions::SEC_ID_TYPE, "It does not support sec_id_type and sec_id parameters")?
    }

    if contract.combo_legs.iter().any(|combo_leg| combo_leg.exempt_code != -1) {
        client.check_server_version(server_versions::SSHORTX, "It does not support exempt_code parameter")?
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support trading_class parameters in place_order",
        )?
    }

    Ok(())
}
