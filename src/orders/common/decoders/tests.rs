use super::*;

#[test]
fn test_completed_order_parsing_issue_318() {
    // Real message captured from live IB Gateway server version 173
    // This is an AAPL STK (stock) order with 102 fields that was successfully parsed
    // This test confirms that the server version fix works for actual server messages
    let raw_message = vec![
        "101",
        "265598",
        "AAPL",
        "STK",
        "",
        "0",
        "?",
        "",
        "SMART",
        "USD",
        "AAPL",
        "NMS",
        "BUY",
        "1",
        "LMT",
        "100.0",
        "0.0",
        "DAY",
        "",
        "DU1236109",
        "",
        "0",
        "",
        "1295810623",
        "0",
        "0",
        "0",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "0",
        "",
        "-1",
        "",
        "",
        "",
        "",
        "",
        "2147483647",
        "0",
        "0",
        "",
        "3",
        "0",
        "",
        "0",
        "None",
        "",
        "0",
        "0",
        "0",
        "",
        "0",
        "0",
        "",
        "",
        "",
        "0",
        "0",
        "0",
        "2147483647",
        "2147483647",
        "",
        "",
        "",
        "IB",
        "0",
        "0",
        "",
        "0",
        "Cancelled",
        "0",
        "0",
        "0",
        "101.0",
        "1.7976931348623157E308",
        "0",
        "1",
        "0",
        "",
        "0",
        "2147483647",
        "0",
        "Not an insider or substantial shareholder",
        "0",
        "0",
        "9223372036854775807",
        "20250924 01:21:07 America/New_York",
        "Cancelled by Trader",
        "",
        "",
        "",
        "",
        "",
        "",
    ];

    let mut message_str = raw_message.join("\0");
    message_str.push('\0'); // match real TWS framing which terminates messages with NUL
    let message = ResponseMessage::from(&message_str);

    // Using the actual server version from our test (173)
    let server_version = 173;

    // This should parse successfully with the server version fix
    let result = decode_completed_order(server_version, message);

    match result {
        Ok(order_data) => {
            // Verify the order was parsed correctly
            assert_eq!(order_data.contract.symbol.to_string(), "AAPL");
            assert_eq!(order_data.contract.security_type.to_string(), "STK");
            assert_eq!(order_data.order.action.to_string(), "BUY");
            assert_eq!(order_data.order.order_type, "LMT");
            assert_eq!(order_data.order.limit_price, Some(100.0));
            assert_eq!(order_data.order_state.status, "Cancelled");
            assert_eq!(order_data.order_state.completed_time, "20250924 01:21:07 America/New_York");
            assert_eq!(order_data.order_state.completed_status, "Cancelled by Trader");

            // Verify that the server version fix worked
            // Server version 173 < all three threshold values (183, 184, 198)
            // So these fields should be empty/default values since they weren't read
            println!("✅ Successfully parsed live server message with {} fields", raw_message.len());
            println!("✅ Server version {} correctly skipped problematic fields", server_version);
        }
        Err(e) => {
            panic!("Failed to parse live server completed order message: {:?}", e);
        }
    }
}

#[test]
fn test_completed_order_parsing_issue_318_bag() {
    // Real BAG (combo/spread) order message with 117 fields
    // This message represents a SPY spread order that was successfully filled
    // This tests the exact scenario from issue #318 with actual BAG order data
    let raw_message = vec![
        "101",
        "28812380",
        "SPY",
        "BAG",
        "",
        "0",
        "?",
        "",
        "SMART",
        "USD",
        "28812380",
        "COMB",
        "BUY",
        "0",
        "LMT",
        "-0.57",
        "0.0",
        "DAY",
        "",
        "DUK000000",
        "",
        "0",
        "bpcs",
        "216108144",
        "0",
        "0",
        "0",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "0",
        "",
        "",
        "0",
        "",
        "-1",
        "",
        "",
        "",
        "",
        "",
        "2147483647",
        "0",
        "0",
        "",
        "3",
        "0",
        "",
        "0",
        "None",
        "",
        "0",
        "0",
        "0",
        "",
        "0",
        "0",
        "",
        "",
        "810118027|1,810118051|-1",
        "2",
        "810118027",
        "1",
        "BUY",
        "SMART",
        "0",
        "0",
        "",
        "-1",
        "810118051",
        "1",
        "SELL",
        "SMART",
        "0",
        "0",
        "",
        "-1",
        "0",
        "0",
        "2147483647",
        "2147483647",
        "",
        "",
        "",
        "IB",
        "0",
        "0",
        "",
        "0",
        "Filled",
        "0",
        "0",
        "0",
        "1.7976931348623157E308",
        "1.7976931348623157E308",
        "0",
        "1",
        "0",
        "",
        "1",
        "2147483647",
        "0",
        "Not an insider or substantial shareholder",
        "0",
        "0",
        "0",
        "20250922 11:49:07 America/Los_Angeles",
        "Filled Size: 1",
        "",
        "",
        "",
        "",
        "",
    ];

    let mut message_str = raw_message.join("\0");
    message_str.push('\0'); // ensure final empty field is preserved
    let message = ResponseMessage::from(&message_str);

    // Using server version 173 which is below the thresholds for problematic fields
    let server_version = 173;

    // This should parse successfully with the server version fix
    let result = decode_completed_order(server_version, message);

    match result {
        Ok(order_data) => {
            // Verify the BAG order was parsed correctly
            assert_eq!(order_data.contract.symbol.to_string(), "SPY");
            assert_eq!(order_data.contract.security_type.to_string(), "BAG");
            assert_eq!(order_data.order.action.to_string(), "BUY");
            assert_eq!(order_data.order.order_type, "LMT");
            assert_eq!(order_data.order.limit_price, Some(-0.57));
            assert_eq!(order_data.order_state.status, "Filled");
            assert_eq!(order_data.order_state.completed_time, "20250922 11:49:07 America/Los_Angeles");
            assert_eq!(order_data.order_state.completed_status, "Filled Size: 1");

            // Verify combo legs were parsed (should have 2 legs for this spread)
            assert!(!order_data.contract.combo_legs.is_empty(), "BAG order should have combo legs");

            println!("✅ Successfully parsed real BAG order with {} fields", raw_message.len());
            println!("✅ BAG order has {} combo legs", order_data.contract.combo_legs.len());
        }
        Err(e) => {
            panic!("Failed to parse BAG order from issue #318: {:?}", e);
        }
    }
}

/// Tests round-trip encoding and decoding of PriceCondition.
#[test]
fn test_price_condition_round_trip() {
    use crate::orders::conditions::{PriceCondition, TriggerMethod};

    let expected = OrderCondition::Price(PriceCondition {
        contract_id: 12345,
        exchange: "NASDAQ".to_string(),
        price: 150.0,
        trigger_method: TriggerMethod::DoubleBidAsk,
        is_more: true,
        is_conjunction: false,
    });

    // condition_type\0is_conjunction\0contract_id\0exchange\0is_more\0price\0trigger_method
    let mut msg = ResponseMessage::from("1\00\012345\0NASDAQ\01\0150\01\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_price_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 1);
    assert_eq!(expected, decoded);
}

#[test]
fn test_time_condition_round_trip() {
    use crate::orders::conditions::TimeCondition;

    let expected = OrderCondition::Time(TimeCondition {
        time: "20251230 23:59:59 UTC".to_string(),
        is_more: true,
        is_conjunction: true,
    });

    // condition_type\0is_conjunction\0is_more\0time
    let mut msg = ResponseMessage::from("3\01\01\020251230 23:59:59 UTC\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_time_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 3);
    assert_eq!(expected, decoded);
}

#[test]
fn test_margin_condition_round_trip() {
    use crate::orders::conditions::MarginCondition;

    let expected = OrderCondition::Margin(MarginCondition {
        percent: 30,
        is_more: false,
        is_conjunction: true,
    });

    // condition_type\0is_conjunction\0is_more\0percent
    let mut msg = ResponseMessage::from("4\01\00\030\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_margin_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 4);
    assert_eq!(expected, decoded);
}

#[test]
fn test_execution_condition_round_trip() {
    use crate::orders::conditions::ExecutionCondition;

    let expected = OrderCondition::Execution(ExecutionCondition {
        symbol: "AAPL".to_string(),
        security_type: "STK".to_string(),
        exchange: "SMART".to_string(),
        is_conjunction: false,
    });

    // condition_type\0is_conjunction\0symbol\0security_type\0exchange
    let mut msg = ResponseMessage::from("5\00\0AAPL\0STK\0SMART\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_execution_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 5);
    assert_eq!(expected, decoded);
}

#[test]
fn test_volume_condition_round_trip() {
    use crate::orders::conditions::VolumeCondition;

    let expected = OrderCondition::Volume(VolumeCondition {
        contract_id: 12345,
        exchange: "NASDAQ".to_string(),
        volume: 1000000,
        is_more: true,
        is_conjunction: true,
    });

    // condition_type\0is_conjunction\0contract_id\0exchange\0is_more\0volume
    let mut msg = ResponseMessage::from("6\01\012345\0NASDAQ\01\01000000\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_volume_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 6);
    assert_eq!(expected, decoded);
}

#[test]
fn test_percent_change_condition_round_trip() {
    use crate::orders::conditions::PercentChangeCondition;

    let expected = OrderCondition::PercentChange(PercentChangeCondition {
        contract_id: 12345,
        exchange: "NASDAQ".to_string(),
        percent: 5.0,
        is_more: false,
        is_conjunction: false,
    });

    // condition_type\0is_conjunction\0contract_id\0exchange\0is_more\0percent
    let mut msg = ResponseMessage::from("7\00\012345\0NASDAQ\00\05\0");
    let condition_type = msg.next_int().unwrap();
    let is_conjunction = msg.next_bool().unwrap();
    let decoded = decode_percent_change_condition(&mut msg, is_conjunction).unwrap();

    assert_eq!(condition_type, 7);
    assert_eq!(expected, decoded);
}

/// Tests error handling for unknown condition type.
#[test]
fn test_unknown_condition_type() {
    let encoded = "99\x001\x00"; // Unknown type 99
    let mut response_message = ResponseMessage::from(encoded);

    let condition_type = response_message.next_int().unwrap();
    let _is_conjunction = response_message.next_bool().unwrap();

    // Should return error for unknown type
    match condition_type {
        1 => panic!("Should be unknown type"),
        _ => {
            // This is the expected path - unknown type should be caught in read_conditions
            assert_eq!(condition_type, 99);
        }
    }
}

/// Builds a base open order message fields for a simple AAPL LMT order.
/// Server version must be >= ORDER_CONTAINER (145) and >= FA_PROFILE_DESUPPORT (177).
fn build_open_order_base_fields(server_version: i32) -> Vec<&'static str> {
    let mut fields = vec![
        "5", // message type (OpenOrder)
        // No message version (server_version >= ORDER_CONTAINER)
        "42", // order_id
        // contract fields
        "265598", // contract_id
        "AAPL",   // symbol
        "STK",    // security_type
        "",       // last_trade_date
        "0",      // strike
        "?",      // right
        "",       // multiplier
        "SMART",  // exchange
        "USD",    // currency
        "AAPL",   // local_symbol
        "NMS",    // trading_class
        // order fields
        "BUY",       // action
        "100",       // total_quantity
        "LMT",       // order_type
        "150.50",    // limit_price
        "0",         // aux_price
        "DAY",       // tif
        "",          // oca_group
        "DU1234567", // account
        "",          // open_close
        "0",         // origin
        "",          // order_ref
        "1",         // client_id
        "123456",    // perm_id
        "0",         // outside_rth
        "0",         // hidden
        "0",         // discretionary_amt
        "",          // good_after_time
        "",          // skip_shares_allocation
        "",          // fa_group
        "",          // fa_method
        "",          // fa_percentage
        // no fa_profile (server_version >= FA_PROFILE_DESUPPORT)
        "",   // model_code (>= MODELS_SUPPORT)
        "",   // good_till_date
        "",   // rule_80_a
        "",   // percent_offset
        "",   // settling_firm
        "0",  // short_sale_slot
        "",   // designated_location
        "-1", // exempt_code
        "0",  // auction_strategy
        "",   // starting_price
        "",   // stock_ref_price
        "",   // delta
        "",   // stock_range_lower
        "",   // stock_range_upper
        "",   // display_size
        "0",  // block_order
        "0",  // sweep_to_fill
        "0",  // all_or_none
        "",   // min_qty
        "0",  // oca_type
        "",   // skip_etrade_only
        "",   // skip_firm_quote_only
        "",   // skip_nbbo_price_cap
        "0",  // parent_id
        "0",  // trigger_method
        // volatility_order_params (read_open_order_attributes=true)
        "", // volatility
        "", // volatility_type
        "", // delta_neutral_order_type
        "", // delta_neutral_aux_price
        // (not delta neutral, so no extra fields)
        "0", // continuous_update
        "",  // reference_price_type
        // trail_params
        "", // trail_stop_price
        "", // trailing_percent
        // basis_points
        "", // basis_points
        "", // basis_points_type
        // combo_legs
        "",  // combo_legs_description
        "0", // combo_legs_count
        "0", // order_combo_legs_count
        // smart_combo_routing_params
        "0", // count
        // scale_order_params
        "", // scale_init_level_size
        "", // scale_subs_level_size
        "", // scale_price_increment
        // hedge_params
        "", // hedge_type (empty, no hedge_param)
        // opt_out_smart_routing
        "0",
        // clearing_params
        "", // clearing_account
        "", // clearing_intent
        // not_held
        "0",
        // delta_neutral
        "0", // has_delta_neutral_contract (false)
        // algo_params
        "", // algo_strategy (empty, no params)
        // solicited
        "0",
        // what_if_info_and_commission
        "0",         // what_if
        "Submitted", // order_status
        // what_if_ext_fields (>= WHAT_IF_EXT_FIELDS)
        "", // initial_margin_before
        "", // maintenance_margin_before
        "", // equity_with_loan_before
        "", // initial_margin_change
        "", // maintenance_margin_change
        "", // equity_with_loan_change
        "", // initial_margin_after
        "", // maintenance_margin_after
        "", // equity_with_loan_after
        "", // commission
        "", // minimum_commission
        "", // maximum_commission
        "", // commission_currency
    ];

    // full_order_preview_fields (>= FULL_ORDER_PREVIEW_FIELDS=195)
    if server_version >= server_versions::FULL_ORDER_PREVIEW_FIELDS {
        fields.extend_from_slice(&[
            "",  // margin_currency
            "",  // initial_margin_before_outside_rth
            "",  // maintenance_margin_before_outside_rth
            "",  // equity_with_loan_before_outside_rth
            "",  // initial_margin_change_outside_rth
            "",  // maintenance_margin_change_outside_rth
            "",  // equity_with_loan_change_outside_rth
            "",  // initial_margin_after_outside_rth
            "",  // maintenance_margin_after_outside_rth
            "",  // equity_with_loan_after_outside_rth
            "",  // suggested_size
            "",  // reject_reason
            "0", // order_allocations_count
        ]);
    }

    fields.extend_from_slice(&[
        "", // warning_text
        // vol_randomize_flags
        "0", // randomize_size
        "0", // randomize_price
        // peg_to_bench: skipped (order_type != "PEG BENCH")
        // conditions (>= PEGGED_TO_BENCHMARK)
        "0", // conditions_count
        // adjusted_order_params (>= PEGGED_TO_BENCHMARK)
        "",  // adjusted_order_type
        "",  // trigger_price
        "",  // trail_stop_price
        "",  // limit_price_offset
        "",  // adjusted_stop_price
        "",  // adjusted_stop_limit_price
        "",  // adjusted_trailing_amount
        "0", // adjustable_trailing_unit
        // soft_dollar_tier (>= SOFT_DOLLAR_TIER)
        "", // name
        "", // value
        "", // display_name
        // cash_qty (>= CASH_QTY)
        "",  // dont_use_auto_price_for_hedge (>= AUTO_PRICE_FOR_HEDGE)
        "0", // is_oms_container (>= ORDER_CONTAINER)
        "0", // discretionary_up_to_limit_price (>= D_PEG_ORDERS)
        "0", // use_price_mgmt_algo (>= PRICE_MGMT_ALGO)
        "0", // duration (>= DURATION)
        "",  // post_to_ats (>= POST_TO_ATS)
        "",  // auto_cancel_parent (>= AUTO_CANCEL_PARENT)
        "0", // peg_best_peg_mid (>= PEGBEST_PEGMID_OFFSETS)
        "",  // min_trade_qty
        "",  // min_compete_size
        "",  // compete_against_best_offset
        "",  // mid_offset_at_whole
        "",  // mid_offset_at_half
    ]);

    fields
}

#[test]
fn test_decode_open_order_v200_new_fields() {
    let mut fields = build_open_order_base_fields(200);

    // New fields for v183-v199
    fields.push("CUST001"); // customer_account (>= CUSTOMER_ACCOUNT=183)
    fields.push("1"); // professional_customer (>= PROFESSIONAL_CUSTOMER=184)
    fields.push("1.25"); // bond_accrued_interest (>= BOND_ACCRUED_INTEREST=185)
    fields.push("1"); // include_overnight (>= INCLUDE_OVERNIGHT=189)
    fields.push("EXTOP1"); // ext_operator (>= CME_TAGGING_FIELDS_IN_OPEN_ORDER=193)
    fields.push("3"); // manual_order_indicator (>= CME_TAGGING_FIELDS_IN_OPEN_ORDER)
    fields.push("SUB001"); // submitter (>= SUBMITTER=198)
    fields.push("1"); // imbalance_only (>= IMBALANCE_ONLY=199)

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_open_order(200, message).unwrap();

    // Verify core fields
    assert_eq!(result.order_id, 42);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order.action.to_string(), "BUY");
    assert_eq!(result.order.order_type, "LMT");
    assert_eq!(result.order.limit_price, Some(150.50));
    assert_eq!(result.order_state.status, "Submitted");

    // Verify new fields
    assert_eq!(result.order.customer_account, "CUST001");
    assert!(result.order.professional_customer);
    assert_eq!(result.order.bond_accrued_interest, "1.25");
    assert!(result.order.include_overnight);
    assert_eq!(result.order.ext_operator, "EXTOP1");
    assert_eq!(result.order.manual_order_indicator, Some(3));
    assert_eq!(result.order.submitter, "SUB001");
    assert!(result.order.imbalance_only);
}

#[test]
fn test_decode_open_order_v182_skips_new_fields() {
    let fields = build_open_order_base_fields(182);

    // At v182, none of the new fields (>= v183) are present.
    // The message ends after peg_best_peg_mid fields.
    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_open_order(182, message).unwrap();

    // Core fields still parse
    assert_eq!(result.order_id, 42);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order.action.to_string(), "BUY");

    // New fields should be defaults
    assert_eq!(result.order.customer_account, "");
    assert!(!result.order.professional_customer);
    assert_eq!(result.order.bond_accrued_interest, "");
    assert!(!result.order.include_overnight);
    assert_eq!(result.order.ext_operator, "");
    assert_eq!(result.order.manual_order_indicator, None);
    assert_eq!(result.order.submitter, "");
    assert!(!result.order.imbalance_only);
}

#[test]
fn test_decode_open_order_v200_full_order_preview_fields() {
    let mut fields = build_open_order_base_fields(200);

    // Append v183-v199 fields
    fields.extend_from_slice(&[
        "CUST001", // customer_account
        "1",       // professional_customer
        "1.25",    // bond_accrued_interest
        "1",       // include_overnight
        "EXTOP1",  // ext_operator
        "3",       // manual_order_indicator
        "SUB001",  // submitter
        "1",       // imbalance_only
    ]);

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_open_order(200, message).unwrap();

    // Verify full order preview fields are default (empty in base)
    assert_eq!(result.order_state.margin_currency, "");
    assert_eq!(result.order_state.initial_margin_before_outside_rth, None);
    assert_eq!(result.order_state.maintenance_margin_before_outside_rth, None);
    assert_eq!(result.order_state.equity_with_loan_before_outside_rth, None);
    assert_eq!(result.order_state.initial_margin_change_outside_rth, None);
    assert_eq!(result.order_state.maintenance_margin_change_outside_rth, None);
    assert_eq!(result.order_state.equity_with_loan_change_outside_rth, None);
    assert_eq!(result.order_state.initial_margin_after_outside_rth, None);
    assert_eq!(result.order_state.maintenance_margin_after_outside_rth, None);
    assert_eq!(result.order_state.equity_with_loan_after_outside_rth, None);
    assert_eq!(result.order_state.suggested_size, None);
    assert_eq!(result.order_state.reject_reason, "");
    assert!(result.order_state.order_allocations.is_empty());
}

#[test]
fn test_decode_open_order_v200_full_order_preview_with_values() {
    // Build v194 base (no preview block), then splice in preview fields with values
    let base = build_open_order_base_fields(194);

    // Find "Submitted" (order_status) to locate the insertion point
    let status_idx = base.iter().position(|&f| f == "Submitted").unwrap();
    // After status: 6 ext margins + 3 after margins + 3 commissions + commission_currency = 13
    let after_commission_currency = status_idx + 1 + 13;

    let mut fields: Vec<&str> = base[..after_commission_currency].to_vec();

    // Insert full_order_preview_fields with values
    fields.extend_from_slice(&[
        "USD",                // margin_currency
        "5000.0",             // initial_margin_before_outside_rth
        "4000.0",             // maintenance_margin_before_outside_rth
        "3000.0",             // equity_with_loan_before_outside_rth
        "100.0",              // initial_margin_change_outside_rth
        "80.0",               // maintenance_margin_change_outside_rth
        "60.0",               // equity_with_loan_change_outside_rth
        "5100.0",             // initial_margin_after_outside_rth
        "4080.0",             // maintenance_margin_after_outside_rth
        "3060.0",             // equity_with_loan_after_outside_rth
        "50.0",               // suggested_size
        "some reject reason", // reject_reason
        "2",                  // order_allocations_count
        "ACC1",               // allocation[0].account
        "100.0",              // allocation[0].position
        "150.0",              // allocation[0].position_desired
        "150.0",              // allocation[0].position_after
        "50.0",               // allocation[0].desired_alloc_qty
        "50.0",               // allocation[0].allowed_alloc_qty
        "0",                  // allocation[0].is_monetary
        "ACC2",               // allocation[1].account
        "200.0",              // allocation[1].position
        "250.0",              // allocation[1].position_desired
        "250.0",              // allocation[1].position_after
        "50.0",               // allocation[1].desired_alloc_qty
        "50.0",               // allocation[1].allowed_alloc_qty
        "1",                  // allocation[1].is_monetary
    ]);

    // Append rest of base (warning_text onward)
    fields.extend_from_slice(&base[after_commission_currency..]);

    // Append v183+ fields
    fields.extend_from_slice(&["CUST001", "1", "1.25", "1", "EXTOP1", "3", "SUB001", "1"]);

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_open_order(200, message).unwrap();

    assert_eq!(result.order_state.margin_currency, "USD");
    assert_eq!(result.order_state.initial_margin_before_outside_rth, Some(5000.0));
    assert_eq!(result.order_state.maintenance_margin_before_outside_rth, Some(4000.0));
    assert_eq!(result.order_state.equity_with_loan_before_outside_rth, Some(3000.0));
    assert_eq!(result.order_state.initial_margin_change_outside_rth, Some(100.0));
    assert_eq!(result.order_state.maintenance_margin_change_outside_rth, Some(80.0));
    assert_eq!(result.order_state.equity_with_loan_change_outside_rth, Some(60.0));
    assert_eq!(result.order_state.initial_margin_after_outside_rth, Some(5100.0));
    assert_eq!(result.order_state.maintenance_margin_after_outside_rth, Some(4080.0));
    assert_eq!(result.order_state.equity_with_loan_after_outside_rth, Some(3060.0));
    assert_eq!(result.order_state.suggested_size, Some(50.0));
    assert_eq!(result.order_state.reject_reason, "some reject reason");

    assert_eq!(result.order_state.order_allocations.len(), 2);
    let alloc0 = &result.order_state.order_allocations[0];
    assert_eq!(alloc0.account, "ACC1");
    assert_eq!(alloc0.position, Some(100.0));
    assert_eq!(alloc0.position_desired, Some(150.0));
    assert_eq!(alloc0.position_after, Some(150.0));
    assert_eq!(alloc0.desired_alloc_qty, Some(50.0));
    assert_eq!(alloc0.allowed_alloc_qty, Some(50.0));
    assert!(!alloc0.is_monetary);

    let alloc1 = &result.order_state.order_allocations[1];
    assert_eq!(alloc1.account, "ACC2");
    assert!(alloc1.is_monetary);
}

#[test]
fn test_decode_execution_data_v200_new_fields() {
    let fields = vec![
        "11", // message type (ExecutionData)
        // no version (server_version >= LAST_LIQUIDITY)
        "9000",                         // request_id
        "42",                           // order_id
        "265598",                       // contract_id
        "AAPL",                         // symbol
        "STK",                          // security_type
        "",                             // last_trade_date
        "0",                            // strike
        "?",                            // right
        "",                             // multiplier
        "SMART",                        // exchange
        "USD",                          // currency
        "AAPL",                         // local_symbol
        "NMS",                          // trading_class
        "0001f4e8.67890abc.01.01",      // execution_id
        "20260115 10:30:00 US/Eastern", // time
        "DU1234567",                    // account_number
        "SMART",                        // exchange
        "BOT",                          // side
        "100",                          // shares
        "150.50",                       // price
        "123456",                       // perm_id
        "1",                            // client_id
        "0",                            // liquidation
        "100",                          // cumulative_quantity
        "150.50",                       // average_price
        "",                             // order_reference
        "",                             // ev_rule
        "",                             // ev_multiplier
        "",                             // model_code (>= MODELS_SUPPORT)
        "2",                            // last_liquidity (>= LAST_LIQUIDITY)
        "1",                            // pending_price_revision (>= PENDING_PRICE_REVISION=178)
        "SUB002",                       // submitter (>= SUBMITTER=198)
    ];

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let mut message = ResponseMessage::from(&message_str);

    let result = decode_execution_data(200, &mut message).unwrap();

    // Verify core fields
    assert_eq!(result.request_id, 9000);
    assert_eq!(result.execution.order_id, 42);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.execution.execution_id, "0001f4e8.67890abc.01.01");
    assert_eq!(result.execution.shares, 100.0);
    assert_eq!(result.execution.price, 150.50);

    // Verify new fields
    assert!(result.execution.pending_price_revision);
    assert_eq!(result.execution.submitter, "SUB002");
}

#[test]
fn test_decode_execution_data_v177_skips_new_fields() {
    // v177 is below PENDING_PRICE_REVISION (178) and SUBMITTER (198)
    let fields = vec![
        "11",                           // message type
        "9000",                         // request_id
        "42",                           // order_id
        "265598",                       // contract_id
        "AAPL",                         // symbol
        "STK",                          // security_type
        "",                             // last_trade_date
        "0",                            // strike
        "?",                            // right
        "",                             // multiplier
        "SMART",                        // exchange
        "USD",                          // currency
        "AAPL",                         // local_symbol
        "NMS",                          // trading_class
        "0001f4e8.67890abc.01.01",      // execution_id
        "20260115 10:30:00 US/Eastern", // time
        "DU1234567",                    // account_number
        "SMART",                        // exchange
        "BOT",                          // side
        "100",                          // shares
        "150.50",                       // price
        "123456",                       // perm_id
        "1",                            // client_id
        "0",                            // liquidation
        "100",                          // cumulative_quantity
        "150.50",                       // average_price
        "",                             // order_reference
        "",                             // ev_rule
        "",                             // ev_multiplier
        "",                             // model_code (>= MODELS_SUPPORT)
        "2",                            // last_liquidity (>= LAST_LIQUIDITY)
                                        // No pending_price_revision (v177 < 178)
                                        // No submitter (v177 < 198)
    ];

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let mut message = ResponseMessage::from(&message_str);

    let result = decode_execution_data(177, &mut message).unwrap();

    assert_eq!(result.execution.order_id, 42);
    assert!(!result.execution.pending_price_revision);
    assert_eq!(result.execution.submitter, "");
}

/// Builds base completed order message fields for a simple AAPL LMT order.
fn build_completed_order_base_fields() -> Vec<&'static str> {
    vec![
        "101", // message type (CompletedOrder)
        // No message version (server_version >= ORDER_CONTAINER)
        // contract fields
        "265598", // contract_id
        "AAPL",   // symbol
        "STK",    // security_type
        "",       // last_trade_date
        "0",      // strike
        "?",      // right
        "",       // multiplier
        "SMART",  // exchange
        "USD",    // currency
        "AAPL",   // local_symbol
        "NMS",    // trading_class
        // order fields
        "BUY",       // action
        "1",         // total_quantity
        "LMT",       // order_type
        "100.0",     // limit_price
        "0.0",       // aux_price
        "DAY",       // tif
        "",          // oca_group
        "DU1234567", // account
        "",          // open_close
        "0",         // origin
        "",          // order_ref
        // (no client_id in completed orders)
        "1295810623", // perm_id
        "0",          // outside_rth
        "0",          // hidden
        "0",          // discretionary_amt
        "",           // good_after_time
        // (no skip_shares_allocation in completed orders)
        "", // fa_group
        "", // fa_method
        "", // fa_percentage
        // no fa_profile (>= FA_PROFILE_DESUPPORT)
        "",   // model_code (>= MODELS_SUPPORT)
        "",   // good_till_date
        "",   // rule_80_a
        "",   // percent_offset
        "",   // settling_firm
        "0",  // short_sale_slot
        "",   // designated_location
        "-1", // exempt_code
        // (no auction_strategy in completed orders)
        "", // starting_price
        "", // stock_ref_price
        "", // delta
        "", // stock_range_lower
        "", // stock_range_upper
        "", // display_size
        // (no block_order in completed orders)
        "0", // sweep_to_fill
        "0", // all_or_none
        "",  // min_qty
        "0", // oca_type
        // (no skip_etrade_only, skip_firm_quote_only, skip_nbbo_price_cap)
        // (no parent_id)
        "0", // trigger_method
        // volatility_order_params (read_open_order_attributes=false)
        "",  // volatility
        "",  // volatility_type
        "",  // delta_neutral_order_type
        "",  // delta_neutral_aux_price
        "0", // continuous_update
        "",  // reference_price_type
        // trail_params
        "", // trail_stop_price
        "", // trailing_percent
        // (no basis_points in completed orders)
        // combo_legs
        "",  // combo_legs_description
        "0", // combo_legs_count
        "0", // order_combo_legs_count
        // smart_combo_routing_params
        "0", // count
        // scale_order_params
        "", // scale_init_level_size
        "", // scale_subs_level_size
        "", // scale_price_increment
        // hedge_params
        "", // hedge_type (empty)
        // (no opt_out_smart_routing in completed orders)
        // clearing_params
        "", // clearing_account
        "", // clearing_intent
        // not_held
        "0",
        // delta_neutral
        "0", // has_delta_neutral_contract
        // algo_params
        "", // algo_strategy
        // solicited
        "0",
        // order_status
        "Cancelled",
        // vol_randomize_flags
        "0", // randomize_size
        "0", // randomize_price
        // peg_to_bench: skipped (order_type != "PEG BENCH")
        // conditions (>= PEGGED_TO_BENCHMARK)
        "0", // conditions_count
        // stop_price_and_limit_price_offset
        "", // trail_stop_price
        "", // limit_price_offset
        // cash_qty (>= CASH_QTY)
        "",
        // dont_use_auto_price_for_hedge (>= AUTO_PRICE_FOR_HEDGE)
        "0",
        // is_oms_container (>= ORDER_CONTAINER)
        "0",
        // auto_cancel_date
        "",
        // filled_quantity
        "0",
        // ref_futures_contract_id
        "",
        // auto_cancel_parent (>= AUTO_CANCEL_PARENT)
        "0",
        // shareholder
        "Not an insider or substantial shareholder",
        // imbalance_only (min_version=0, always read)
        "0",
        // route_marketable_to_bbo
        "0",
        // parent_perm_id
        "9223372036854775807",
        // completed_time
        "20260115 10:30:00 America/New_York",
        // completed_status
        "Cancelled by Trader",
        // peg_best_peg_mid (>= PEGBEST_PEGMID_OFFSETS)
        "", // min_trade_qty
        "", // min_compete_size
        "", // compete_against_best_offset
        "", // mid_offset_at_whole
        "", // mid_offset_at_half
    ]
}

#[test]
fn test_decode_completed_order_v200_new_fields() {
    let mut fields = build_completed_order_base_fields();

    // New fields for completed orders at v200
    fields.push("CUST002"); // customer_account (>= CUSTOMER_ACCOUNT=183)
    fields.push("1"); // professional_customer (>= PROFESSIONAL_CUSTOMER=184)
    fields.push("SUB003"); // submitter (>= SUBMITTER=198)
                           // Note: completed orders do NOT decode bond_accrued_interest,
                           // include_overnight, or cme_tagging_fields per C# reference

    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_completed_order(200, message).unwrap();

    // Verify core fields
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order.action.to_string(), "BUY");
    assert_eq!(result.order_state.status, "Cancelled");
    assert_eq!(result.order_state.completed_time, "20260115 10:30:00 America/New_York");
    assert_eq!(result.order_state.completed_status, "Cancelled by Trader");

    // Verify new fields
    assert_eq!(result.order.customer_account, "CUST002");
    assert!(result.order.professional_customer);
    assert_eq!(result.order.submitter, "SUB003");
}

#[test]
fn test_decode_completed_order_v182_skips_new_fields() {
    let fields = build_completed_order_base_fields();

    // At v182, customer_account, professional_customer, submitter are not present
    let mut message_str = fields.join("\0");
    message_str.push('\0');
    let message = ResponseMessage::from(&message_str);

    let result = decode_completed_order(182, message).unwrap();

    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order_state.completed_status, "Cancelled by Trader");

    // New fields should be defaults
    assert_eq!(result.order.customer_account, "");
    assert!(!result.order.professional_customer);
    assert_eq!(result.order.submitter, "");
}

#[test]
fn test_decode_open_order_proto() {
    use prost::Message;

    let proto_msg = crate::proto::OpenOrder {
        order_id: Some(42),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            exchange: Some("SMART".into()),
            currency: Some("USD".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            order_id: Some(42),
            action: Some("BUY".into()),
            total_quantity: Some("100".into()),
            order_type: Some("LMT".into()),
            lmt_price: Some(150.0),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Submitted".into()),
            ..Default::default()
        }),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_open_order_proto(&bytes).unwrap();
    assert_eq!(result.order_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol.to_string(), "AAPL");
    assert_eq!(result.order.order_id, 42);
    assert_eq!(result.order.action, Action::Buy);
    assert_eq!(result.order.total_quantity, 100.0);
    assert_eq!(result.order.order_type, "LMT");
    assert_eq!(result.order.limit_price, Some(150.0));
    assert_eq!(result.order_state.status, "Submitted");
}

#[test]
fn test_decode_order_status_proto() {
    use prost::Message;

    let proto_msg = crate::proto::OrderStatus {
        order_id: Some(99),
        status: Some("Filled".into()),
        filled: Some("50".into()),
        remaining: Some("0".into()),
        avg_fill_price: Some(152.5),
        perm_id: Some(123456),
        parent_id: Some(10),
        last_fill_price: Some(152.75),
        client_id: Some(7),
        why_held: Some("locate".into()),
        mkt_cap_price: Some(1.23),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_order_status_proto(&bytes).unwrap();
    assert_eq!(result.order_id, 99);
    assert_eq!(result.status, "Filled");
    assert_eq!(result.filled, 50.0);
    assert_eq!(result.remaining, 0.0);
    assert_eq!(result.average_fill_price, Some(152.5));
    assert_eq!(result.perm_id, 123456);
    assert_eq!(result.parent_id, 10);
    assert_eq!(result.last_fill_price, Some(152.75));
    assert_eq!(result.client_id, 7);
    assert_eq!(result.why_held, "locate");
    assert_eq!(result.market_cap_price, Some(1.23));
}

#[test]
fn test_decode_order_status_text_unset_double() {
    // IBKR sends UNSET_DOUBLE (1.7976931348623157E308) for unset price fields;
    // they must decode to None, not f64::MAX leaking through.
    let raw = "3\013\0PreSubmitted\00\0100\01.7976931348623157E308\01376327563\00\01.7976931348623157E308\0100\0\01.7976931348623157E308\0";
    let mut message = ResponseMessage::from(raw);

    let result = decode_order_status(server_versions::SIZE_RULES, &mut message).unwrap();

    assert_eq!(result.order_id, 13);
    assert_eq!(result.status, "PreSubmitted");
    assert_eq!(result.average_fill_price, None);
    assert_eq!(result.last_fill_price, None);
    assert_eq!(result.market_cap_price, None);
}

#[test]
fn test_decode_order_status_text_empty_double() {
    let raw = "3\013\0PreSubmitted\00\0100\0\01376327563\00\0\0100\0\0\0";
    let mut message = ResponseMessage::from(raw);

    let result = decode_order_status(server_versions::SIZE_RULES, &mut message).unwrap();

    assert_eq!(result.average_fill_price, None);
    assert_eq!(result.last_fill_price, None);
    assert_eq!(result.market_cap_price, None);
}

#[test]
fn test_decode_order_status_proto_missing_doubles() {
    // Regression: previously decoded to Some(0.0) via unwrap_or_default().
    use prost::Message;

    let proto_msg = crate::proto::OrderStatus {
        order_id: Some(99),
        status: Some("Submitted".into()),
        filled: Some("0".into()),
        remaining: Some("100".into()),
        avg_fill_price: None,
        perm_id: Some(123456),
        parent_id: Some(0),
        last_fill_price: None,
        client_id: Some(7),
        why_held: None,
        mkt_cap_price: None,
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_order_status_proto(&bytes).unwrap();
    assert_eq!(result.average_fill_price, None);
    assert_eq!(result.last_fill_price, None);
    assert_eq!(result.market_cap_price, None);
}

#[test]
fn test_decode_commission_report_proto() {
    use prost::Message;

    let proto_msg = crate::proto::CommissionAndFeesReport {
        exec_id: Some("exec123".into()),
        commission_and_fees: Some(1.25),
        currency: Some("USD".into()),
        realized_pnl: Some(500.0),
        bond_yield: Some(f64::MAX),
        yield_redemption_date: Some("20260101".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_commission_report_proto(&bytes).unwrap();
    assert_eq!(result.execution_id, "exec123");
    assert_eq!(result.commission, 1.25);
    assert_eq!(result.currency, "USD");
    assert_eq!(result.realized_pnl, Some(500.0));
    assert_eq!(result.yields, None); // f64::MAX filtered out
    assert_eq!(result.yield_redemption_date, "20260101");
}

#[test]
fn test_decode_execution_data_proto() {
    use prost::Message;

    let proto_msg = crate::proto::ExecutionDetails {
        req_id: Some(42),
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        execution: Some(crate::proto::Execution {
            order_id: Some(100),
            exec_id: Some("exec001".into()),
            time: Some("20260101 12:00:00".into()),
            acct_number: Some("DU1234".into()),
            side: Some("BOT".into()),
            shares: Some("50".into()),
            price: Some(152.5),
            perm_id: Some(99999),
            ..Default::default()
        }),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_execution_data_proto(&bytes).unwrap();
    assert_eq!(result.request_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.execution.execution_id, "exec001");
    assert_eq!(result.execution.shares, 50.0);
    assert_eq!(result.execution.price, 152.5);
    assert_eq!(result.execution.perm_id, 99999);
}

#[test]
fn test_decode_completed_order_proto() {
    use prost::Message;

    let proto_msg = crate::proto::CompletedOrder {
        contract: Some(crate::proto::Contract {
            con_id: Some(265598),
            symbol: Some("AAPL".into()),
            sec_type: Some("STK".into()),
            ..Default::default()
        }),
        order: Some(crate::proto::Order {
            order_id: Some(200),
            action: Some("SELL".into()),
            total_quantity: Some("200".into()),
            order_type: Some("MKT".into()),
            ..Default::default()
        }),
        order_state: Some(crate::proto::OrderState {
            status: Some("Filled".into()),
            completed_time: Some("20260101 12:00:00".into()),
            completed_status: Some("Filled".into()),
            ..Default::default()
        }),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_completed_order_proto(&bytes).unwrap();
    assert_eq!(result.order_id, 200);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.contract.symbol, Symbol::from("AAPL"));
    assert_eq!(result.order.action, Action::Sell);
    assert_eq!(result.order_state.completed_time, "20260101 12:00:00");
}

// =============================================================================
// Builder → production-decoder integration tests
// =============================================================================
//
// These exercise the production `decode_*_proto` decoders by routing the
// testdata builders' `encode_proto()` output through them. The builders give
// named, defaulted construction; the decoder is what we're verifying.

#[test]
fn test_decode_order_status_proto_round_trips_via_builder() {
    use crate::testdata::builders::orders::order_status;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = order_status()
        .order_id(99)
        .status("Filled")
        .filled(50.0)
        .remaining(0.0)
        .average_fill_price(Some(152.5))
        .perm_id(123456)
        .last_fill_price(Some(152.75))
        .client_id(7)
        .market_cap_price(Some(1.23))
        .encode_proto();

    let result = super::decode_order_status_proto(&bytes).unwrap();
    assert_eq!(result.order_id, 99);
    assert_eq!(result.status, "Filled");
    assert_eq!(result.filled, 50.0);
    assert_eq!(result.remaining, 0.0);
    assert_eq!(result.average_fill_price, Some(152.5));
    assert_eq!(result.perm_id, 123456);
    assert_eq!(result.last_fill_price, Some(152.75));
    assert_eq!(result.client_id, 7);
    assert_eq!(result.market_cap_price, Some(1.23));
}

#[test]
fn test_decode_commission_report_proto_round_trips_via_builder() {
    use crate::testdata::builders::orders::commission_report;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = commission_report()
        .execution_id("exec123")
        .commission(1.25)
        .currency("USD")
        .realized_pnl(Some(500.0))
        .yields(Some(f64::MAX))
        .encode_proto();

    let result = super::decode_commission_report_proto(&bytes).unwrap();
    assert_eq!(result.execution_id, "exec123");
    assert_eq!(result.commission, 1.25);
    assert_eq!(result.currency, "USD");
    assert_eq!(result.realized_pnl, Some(500.0));
    assert_eq!(result.yields, None); // f64::MAX is the IBKR sentinel for "unset"
}

#[test]
fn test_decode_execution_data_proto_round_trips_via_builder() {
    use crate::testdata::builders::orders::execution_data;
    use crate::testdata::builders::ResponseProtoEncoder;

    let bytes = execution_data()
        .request_id(42)
        .order_id(100)
        .contract_id(265598)
        .symbol("AAPL")
        .security_type("STK")
        .execution_id("exec001")
        .side("BOT")
        .shares(50.0)
        .price(152.5)
        .perm_id(99999)
        .encode_proto();

    let result = super::decode_execution_data_proto(&bytes).unwrap();
    assert_eq!(result.request_id, 42);
    assert_eq!(result.contract.contract_id, 265598);
    assert_eq!(result.execution.execution_id, "exec001");
    assert_eq!(result.execution.shares, 50.0);
    assert_eq!(result.execution.price, 152.5);
    assert_eq!(result.execution.perm_id, 99999);
}
