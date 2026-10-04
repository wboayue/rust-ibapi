use super::*;
use crate::contracts::{Contract, Currency, Exchange, Symbol};
use crate::market_data::TradingHours;
use crate::orders::conditions::TriggerMethod;
use crate::orders::{Action, OcaType, OrderOpenClose, OrderOrigin, ReferencePriceType, Rule80A, ShortSaleSlot, TimeInForce, VolatilityType};
use crate::proto::encoders::encode_order;

fn create_test_contract() -> Contract {
    Contract {
        symbol: Symbol::from("TEST"),
        security_type: crate::contracts::SecurityType::Stock,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    }
}

struct MockClient;

#[test]
fn test_stop_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(95.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "STP");
    assert_eq!(order.aux_price, Some(95.50));
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
}

#[test]
fn test_trailing_stop_limit() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .sell(100)
        .trailing_stop_limit(TrailBy::Percent(5.0), 95.0, 0.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "TRAIL LIMIT");
    assert_eq!(order.trailing_percent, Some(5.0));
    assert_eq!(order.trail_stop_price, Some(95.0));
    assert_eq!(order.limit_price_offset, Some(0.50));
}

#[test]
fn test_market_if_touched() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_if_touched(99.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIT");
    assert_eq!(order.aux_price, Some(99.50));
}

#[test]
fn test_limit_if_touched() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_if_touched(99.50, 100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LIT");
    assert_eq!(order.aux_price, Some(99.50));
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_market_to_limit() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_to_limit();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MTL");
}

#[test]
fn test_block_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).block(50.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.limit_price, Some(50.00));
    assert!(order.block_order);
}

#[test]
fn test_relative_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).relative(0.05, Some(100.00));

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "REL");
    assert_eq!(order.aux_price, Some(0.05));
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_passive_relative() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).passive_relative(0.05);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "PASSV REL");
    assert_eq!(order.aux_price, Some(0.05));
}

#[test]
fn test_midprice_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).midprice(Some(50.00));

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIDPRICE");
    assert_eq!(order.limit_price, Some(50.00));
}

#[test]
fn test_midprice_order_without_price_cap() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).midprice(None);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MIDPRICE");
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_at_auction() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).at_auction(100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MTL");
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_discretionary_amount() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).discretionary(50.00, 0.25);

    let order = builder.build().unwrap();
    assert_eq!(order.limit_price, Some(50.00));
    assert_eq!(order.discretionary_amt, 0.25);
}

#[test]
fn test_sweep_to_fill() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).sweep_to_fill(50.00);

    let order = builder.build().unwrap();
    assert!(order.sweep_to_fill);
    assert_eq!(order.limit_price, Some(50.00));
}

#[test]
fn test_time_conditions() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test Day order
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().day_order();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::Day);

    // Test Good Till Cancel
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().good_till_canceled();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::GoodTillCanceled);

    // Test Immediate or Cancel
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().immediate_or_cancel();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::ImmediateOrCancel);

    // Test Fill or Kill
    let builder = OrderBuilder::new(&client, &contract).buy(100).market().fill_or_kill();

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::FillOrKill);
}

#[test]
fn test_time_in_force_method() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .time_in_force(TimeInForce::ImmediateOrCancel);

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::ImmediateOrCancel);
}

// Every variant reaches the wire under its TWS identifier. GTX previously fell
// through a string round trip and was sent as DAY. A new variant fails to
// compile in `orders::tests::all_tifs_covers_every_variant`, which is the
// reminder to extend this table as well.
#[test]
fn time_in_force_variants_encode_to_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let cases = [
        (TimeInForce::Day, "DAY"),
        (TimeInForce::GoodTillCanceled, "GTC"),
        (TimeInForce::ImmediateOrCancel, "IOC"),
        (TimeInForce::GoodTillDate, "GTD"),
        (TimeInForce::OnOpen, "OPG"),
        (TimeInForce::FillOrKill, "FOK"),
        (TimeInForce::DayTillCanceled, "DTC"),
        (TimeInForce::Auction, "AUC"),
        (TimeInForce::GoodTillCrossing, "GTX"),
    ];

    for (tif, wire) in cases {
        let mut builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.0).time_in_force(tif.clone());
        if tif == TimeInForce::GoodTillDate {
            builder = builder.good_till_time("20240630 23:59:59");
        }
        let order = builder.build().unwrap();
        assert_eq!(order.tif, tif, "tif for {wire}");
        assert_eq!(encode_order(&order).tif.as_deref(), Some(wire), "wire for {wire}");
    }
}

#[test]
fn good_till_crossing_sets_gtx() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .good_till_crossing()
        .build()
        .unwrap();
    assert_eq!(order.tif, TimeInForce::GoodTillCrossing);
    assert_eq!(encode_order(&order).tif.as_deref(), Some("GTX"));
}

#[test]
fn day_till_canceled_sets_dtc() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .day_till_canceled()
        .build()
        .unwrap();
    assert_eq!(order.tif, TimeInForce::DayTillCanceled);
    assert_eq!(encode_order(&order).tif.as_deref(), Some("DTC"));
}

#[test]
fn unknown_time_in_force_reaches_the_wire_unchanged() {
    // An order read back from TWS carrying a TIF this crate does not model can
    // be resubmitted: the raw string passes through the builder untouched
    // rather than being coerced to DAY.
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .time_in_force(TimeInForce::Unknown("GTZ".to_string()))
        .build()
        .unwrap();
    assert_eq!(encode_order(&order).tif.as_deref(), Some("GTZ"));
}

// good_till_date and good_till_time write the same field; the last call wins.
#[test]
fn good_till_date_last_write_wins() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .good_till_date("20240630 23:59:59")
        .good_till_time("20240701 23:59:59")
        .build()
        .unwrap();
    assert_eq!(order.tif, TimeInForce::GoodTillDate);
    assert_eq!(order.good_till_date, "20240701 23:59:59");

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .good_till_time("20240701 23:59:59")
        .good_till_date("20240630 23:59:59")
        .build()
        .unwrap();
    assert_eq!(order.tif, TimeInForce::GoodTillDate);
    assert_eq!(order.good_till_date, "20240630 23:59:59");
}

#[test]
fn good_till_date_without_a_date_fails_validation() {
    let client = MockClient;
    let contract = create_test_contract();

    let err = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .time_in_force(TimeInForce::GoodTillDate)
        .build()
        .unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("good_till_date"));
}

#[test]
fn test_good_till_date() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .good_till_date("20240630 23:59:59");

    let order = builder.build().unwrap();
    assert_eq!(order.tif, TimeInForce::GoodTillDate);
    assert_eq!(order.good_till_date, "20240630 23:59:59");
}

#[test]
fn test_trading_hours_method() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test with Regular hours
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .trading_hours(TradingHours::Regular);

    let order = builder.build().unwrap();
    assert!(!order.outside_rth);

    // Test with Extended hours
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .trading_hours(TradingHours::Extended);

    let order = builder.build().unwrap();
    assert!(order.outside_rth);
}

#[test]
fn test_order_attributes() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).hidden().outside_rth();

    let order = builder.build().unwrap();
    assert!(order.hidden);
    assert!(order.outside_rth);
}

#[test]
fn test_not_held_flag() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().not_held();

    let order = builder.build().unwrap();
    assert!(order.not_held);
}

#[test]
fn test_all_or_none_flag() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().all_or_none();

    let order = builder.build().unwrap();
    assert!(order.all_or_none);
}

#[test]
fn test_account() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market().account("DU123456");

    let order = builder.build().unwrap();
    assert_eq!(order.account, "DU123456");
}

#[test]
fn test_parent_id() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).parent(999);

    let order = builder.build().unwrap();
    assert_eq!(order.parent_id, 999);
}

#[test]
fn test_parent_id_accepts_order_id() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .parent(OrderId::from(7))
        .build()
        .unwrap();
    assert_eq!(order.parent_id, 7);
}

#[test]
fn test_oca_group_settings() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .oca_group("TEST_OCA", OcaType::ReduceWithBlock);

    let order = builder.build().unwrap();
    assert_eq!(order.oca_group, "TEST_OCA");
    assert_eq!(order.oca_type, crate::orders::OcaType::ReduceWithBlock);
}

#[test]
fn oca_type_reaches_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .oca_group("TEST_OCA", OcaType::ReduceWithoutBlock)
        .build()
        .unwrap();

    assert_eq!(encode_order(&order).oca_type, Some(3));
}

#[test]
fn trigger_method_sets_the_field_and_the_wire_code() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .sell(100)
        .stop(95.0)
        .trigger_method(TriggerMethod::Midpoint)
        .build()
        .unwrap();

    assert_eq!(order.trigger_method, TriggerMethod::Midpoint);
    assert_eq!(encode_order(&order).trigger_method, Some(8));
}

#[test]
fn trigger_method_defaults_to_default() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract).sell(100).stop(95.0).build().unwrap();

    assert_eq!(order.trigger_method, TriggerMethod::Default);
    // `Default` is wire code 0, which the encoder omits.
    assert_eq!(encode_order(&order).trigger_method, None);
}

#[test]
fn origin_sets_the_field_and_the_wire_code() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .origin(OrderOrigin::Firm)
        .build()
        .unwrap();

    assert_eq!(order.origin, OrderOrigin::Firm);
    assert_eq!(encode_order(&order).origin, Some(1));
}

#[test]
fn short_sale_slot_and_designated_location_reach_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .sell_short(100)
        .market()
        .short_sale_slot(ShortSaleSlot::ThirdParty)
        .designated_location("ABC SECURITIES")
        .build()
        .unwrap();

    assert_eq!(order.short_sale_slot, ShortSaleSlot::ThirdParty);
    assert_eq!(order.designated_location, "ABC SECURITIES");

    let proto = encode_order(&order);
    assert_eq!(proto.short_sale_slot, Some(2));
    assert_eq!(proto.designated_location.as_deref(), Some("ABC SECURITIES"));
}

#[test]
fn rule_80_a_reaches_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .rule_80_a(Rule80A::AgentOtherMemberPTIA)
        .build()
        .unwrap();

    assert_eq!(order.rule_80_a, Some(Rule80A::AgentOtherMemberPTIA));
    assert_eq!(encode_order(&order).rule80_a.as_deref(), Some("M"));
}

#[test]
fn rule_80_a_is_absent_when_unset() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract).buy(100).limit(50.0).build().unwrap();

    assert_eq!(order.rule_80_a, None);
    assert_eq!(encode_order(&order).rule80_a, None);
}

#[test]
fn open_close_reaches_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .open_close(OrderOpenClose::Close)
        .build()
        .unwrap();

    assert_eq!(order.open_close, Some(OrderOpenClose::Close));
    assert_eq!(encode_order(&order).open_close.as_deref(), Some("C"));
}

#[test]
fn open_close_is_absent_when_unset() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract).buy(100).limit(50.0).build().unwrap();

    assert_eq!(order.open_close, None);
    assert_eq!(encode_order(&order).open_close, None);
}

/// Both enums are open: an `Unknown(raw)` decoded off the wire goes back out
/// unchanged, so round-tripping an inbound order through the builder is lossless.
#[test]
fn open_wire_enums_round_trip_unknown_values_through_the_builder() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .rule_80_a(Rule80A::Unknown("Z".to_string()))
        .open_close(OrderOpenClose::Unknown("X".to_string()))
        .build()
        .unwrap();

    let proto = encode_order(&order);
    assert_eq!(proto.rule80_a.as_deref(), Some("Z"));
    assert_eq!(proto.open_close.as_deref(), Some("X"));
}

/// An `Unknown` carrying the empty string is omitted rather than sent as `""`, matching
/// `action` / `tif` and `EClientUtils`'s `if (!Util.StringIsEmpty(..))`. A decode cannot
/// produce this — `FromStr` rejects empty — but the setters put it one call away.
#[test]
fn empty_unknown_wire_values_are_omitted_not_sent_blank() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .rule_80_a(Rule80A::Unknown(String::new()))
        .open_close(OrderOpenClose::Unknown(String::new()))
        .build()
        .unwrap();

    // The builder keeps what the caller passed; only the encoder drops it.
    assert_eq!(order.rule_80_a, Some(Rule80A::Unknown(String::new())));
    assert_eq!(order.open_close, Some(OrderOpenClose::Unknown(String::new())));

    let proto = encode_order(&order);
    assert_eq!(proto.rule80_a, None);
    assert_eq!(proto.open_close, None);
}

#[test]
fn volatility_type_reaches_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(1)
        .order_type(OrderType::Volatility)
        .volatility(0.25)
        .volatility_type(VolatilityType::Annual)
        .build()
        .unwrap();

    assert_eq!(order.volatility_type, Some(VolatilityType::Annual));
    assert_eq!(encode_order(&order).volatility_type, Some(2));
}

#[test]
fn reference_price_type_reaches_the_wire() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract)
        .buy(1)
        .order_type(OrderType::Volatility)
        .volatility(0.25)
        .reference_price_type(ReferencePriceType::NBBO)
        .build()
        .unwrap();

    assert_eq!(order.reference_price_type, Some(ReferencePriceType::NBBO));
    assert_eq!(encode_order(&order).reference_price_type, Some(2));
}

#[test]
fn volatility_type_does_not_depend_on_volatility_being_set() {
    let client = MockClient;
    let contract = create_test_contract();

    // `build()` used to apply `volatility_type` only inside the `volatility` branch. No
    // caller could reach that guard (the field had no setter until this setter existed),
    // so this is a forward guard, not a regression test: the type is a VOL-order attribute
    // TWS reads on its own, and re-nesting it under `volatility` would drop it silently.
    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.0)
        .volatility_type(VolatilityType::Daily)
        .build()
        .unwrap();

    assert_eq!(order.volatility, None);
    assert_eq!(order.volatility_type, Some(VolatilityType::Daily));
}

#[test]
fn test_algo_order_settings() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .algo("TWAP")
        .algo_param("startTime", "09:30:00")
        .algo_param("endTime", "15:30:00")
        .algo_param("allowPastEndTime", "1");

    let order = builder.build().unwrap();
    assert_eq!(order.algo_strategy, "TWAP");
    assert_eq!(order.algo_params.len(), 3);
    assert_eq!(order.algo_params[0].tag, "startTime");
    assert_eq!(order.algo_params[1].tag, "endTime");
    assert_eq!(order.algo_params[2].tag, "allowPastEndTime");
}

#[test]
fn test_what_if_order() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00).what_if();

    let order = builder.build().unwrap();
    assert!(order.what_if);
}

#[test]
fn test_custom_order_type() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).order_type(OrderType::PegBest);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "PEG BEST");
}

#[test]
fn test_volatility_order_missing_volatility() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).order_type(OrderType::Volatility);
    // Don't set volatility

    let result = builder.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("volatility"));
}

#[test]
fn test_volatility_order_with_volatility() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .order_type(OrderType::Volatility)
        .volatility(0.15);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "VOL");
}

#[test]
fn test_pegged_order_fields() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let order = builder.build().unwrap();
    // Check defaults are properly set
    assert_eq!(order.min_trade_qty, None);
    assert_eq!(order.min_compete_size, None);
    assert_eq!(order.compete_against_best_offset, None);
    assert_eq!(order.mid_offset_at_whole, None);
    assert_eq!(order.mid_offset_at_half, None);
}

#[test]
fn test_validation_errors() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test zero quantity
    let builder = OrderBuilder::new(&client, &contract).buy(0).market();

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::InvalidQuantity(0.0))));

    // Test missing order type
    let builder = OrderBuilder::new(&client, &contract).buy(100);

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::MissingRequiredField("order_type"))));

    // Test invalid stop price (NaN)
    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(f64::NAN);

    let result = builder.build();
    assert!(matches!(result, Err(ValidationError::InvalidPrice(_))));
}

#[test]
fn test_validation_edge_cases() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test with zero stop price (should be valid)
    let builder = OrderBuilder::new(&client, &contract).buy(100).stop(0.0);

    let result = builder.build();
    assert!(result.is_ok());

    // Test limit price of zero (should be valid for some order types)
    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(0.0);

    let result = builder.build();
    assert!(result.is_ok());
}

// ===== Bracket Order Tests =====

#[test]
fn test_bracket_order_build_details() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test Buy bracket order
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify parent order details
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Buy);
    assert_eq!(parent.order_type, "LMT");
    assert_eq!(parent.limit_price, Some(50.0));
    assert!(!parent.transmit);

    // Verify take profit details
    let tp = &orders[1];
    assert_eq!(tp.action, Action::Sell); // Reverse of Buy
    assert_eq!(tp.order_type, "LMT");
    assert_eq!(tp.limit_price, Some(55.0));
    assert_eq!(tp.parent_id, parent.order_id);
    assert!(!tp.transmit);

    // Verify stop loss details
    let sl = &orders[2];
    assert_eq!(sl.action, Action::Sell); // Reverse of Buy
    assert_eq!(sl.order_type, "STP");
    assert_eq!(sl.aux_price, Some(45.0));
    assert_eq!(sl.parent_id, parent.order_id);
    assert!(sl.transmit);
}

#[test]
fn test_bracket_order_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test Sell bracket order
    let bracket = OrderBuilder::new(&client, &contract)
        .sell(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(45.0) // Lower for sell
        .stop_loss(55.0); // Higher for sell

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify actions are reversed for sell bracket
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Sell);

    let tp = &orders[1];
    assert_eq!(tp.action, Action::Buy); // Reverse of Sell

    let sl = &orders[2];
    assert_eq!(sl.action, Action::Buy); // Reverse of Sell
}

#[test]
fn test_bracket_order_validation_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    // Valid buy bracket
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    assert!(bracket.build().is_ok());
}

#[test]
fn test_bracket_order_missing_entry() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract).buy(100).bracket().take_profit(55.0).stop_loss(45.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("entry"));
}

#[test]
fn test_bracket_order_missing_take_profit() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract).buy(100).bracket().entry_limit(50.0).stop_loss(45.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("take_profit"));
}

#[test]
fn test_bracket_order_missing_stop_loss() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("stop_loss"));
}

#[test]
fn test_bracket_order_invalid_prices_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    // Take profit below entry for buy
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(45.0) // Invalid: below entry
        .stop_loss(45.0);

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Take profit (45) must be above entry (50)"));
}

#[test]
fn test_bracket_order_invalid_stop_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    // Stop loss above entry for buy
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(55.0); // Invalid: above entry

    let result = bracket.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Stop loss (55) must be below entry (50)"));
}

#[test]
fn test_bracket_order_large_quantity() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(10000)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders[0].total_quantity, 10000.0);
    assert_eq!(orders[1].total_quantity, 10000.0);
    assert_eq!(orders[2].total_quantity, 10000.0);
}

#[test]
fn test_bracket_order_fractional_prices() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.25)
        .take_profit(55.75)
        .stop_loss(45.50);

    let orders = bracket.build().unwrap();
    assert_eq!(orders[0].limit_price, Some(50.25));
    assert_eq!(orders[1].limit_price, Some(55.75));
    assert_eq!(orders[2].aux_price, Some(45.50));
}

#[test]
fn test_bracket_order_parent_id_propagation() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    let parent_id = orders[0].order_id;

    assert_eq!(orders[1].parent_id, parent_id);
    assert_eq!(orders[2].parent_id, parent_id);
}

#[test]
fn test_bracket_order_transmit_flags() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // Parent and take profit should not transmit
    assert!(!orders[0].transmit);
    assert!(!orders[1].transmit);

    // Stop loss should transmit (last order)
    assert!(orders[2].transmit);
}

#[test]
fn test_bracket_order_action_reversal() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test buy bracket
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders[0].action, Action::Buy);
    assert_eq!(orders[1].action, Action::Sell);
    assert_eq!(orders[2].action, Action::Sell);

    // Test sell bracket
    let bracket = OrderBuilder::new(&client, &contract)
        .sell(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(45.0)
        .stop_loss(55.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders[0].action, Action::Sell);
    assert_eq!(orders[1].action, Action::Buy);
    assert_eq!(orders[2].action, Action::Buy);
}

#[test]
fn test_bracket_order_types() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // Check order types
    assert_eq!(orders[0].order_type, "LMT"); // Parent is limit
    assert_eq!(orders[1].order_type, "LMT"); // Take profit is limit
    assert_eq!(orders[2].order_type, "STP"); // Stop loss is stop
}

#[test]
fn test_bracket_order_inherits_outside_rth() {
    let client = MockClient;
    let contract = create_test_contract();

    // Test with outside_rth enabled
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .outside_rth()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should inherit outside_rth from parent
    assert!(orders[0].outside_rth, "Parent should have outside_rth");
    assert!(orders[1].outside_rth, "Take profit should inherit outside_rth");
    assert!(orders[2].outside_rth, "Stop loss should inherit outside_rth");

    // Test without outside_rth (default)
    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should have outside_rth = false
    assert!(!orders[0].outside_rth);
    assert!(!orders[1].outside_rth);
    assert!(!orders[2].outside_rth);
}

#[test]
fn test_bracket_order_with_missing_action() {
    let client = MockClient;
    let contract = create_test_contract();

    // Create builder without setting action
    let mut builder = OrderBuilder::new(&client, &contract);
    builder.quantity = Some(100.0);

    let bracket = builder.bracket();

    let result = bracket.entry_limit(50.0).take_profit(55.0).stop_loss(45.0).build();

    assert!(result.is_err());
}

// ===== Market Entry Bracket Order Tests =====

#[test]
fn test_bracket_order_market_entry_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify parent order is market order
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Buy);
    assert_eq!(parent.order_type, "MKT");
    assert_eq!(parent.limit_price, None);
    assert!(!parent.transmit);

    // Verify take profit details
    let tp = &orders[1];
    assert_eq!(tp.action, Action::Sell);
    assert_eq!(tp.order_type, "LMT");
    assert_eq!(tp.limit_price, Some(55.0));
    assert_eq!(tp.parent_id, parent.order_id);
    assert!(!tp.transmit);

    // Verify stop loss details
    let sl = &orders[2];
    assert_eq!(sl.action, Action::Sell);
    assert_eq!(sl.order_type, "STP");
    assert_eq!(sl.aux_price, Some(45.0));
    assert_eq!(sl.parent_id, parent.order_id);
    assert!(sl.transmit);
}

#[test]
fn test_bracket_order_market_entry_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .sell(100)
        .bracket()
        .entry_market()
        .take_profit(45.0)
        .stop_loss(55.0);

    let orders = bracket.build().unwrap();
    assert_eq!(orders.len(), 3);

    // Verify parent order is market order with Sell action
    let parent = &orders[0];
    assert_eq!(parent.action, Action::Sell);
    assert_eq!(parent.order_type, "MKT");

    // Verify child orders have reversed action
    let tp = &orders[1];
    assert_eq!(tp.action, Action::Buy);

    let sl = &orders[2];
    assert_eq!(sl.action, Action::Buy);
}

#[test]
fn test_bracket_order_market_entry_inherits_outside_rth() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .outside_rth()
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should inherit outside_rth from parent
    assert!(orders[0].outside_rth, "Parent should have outside_rth");
    assert!(orders[1].outside_rth, "Take profit should inherit outside_rth");
    assert!(orders[2].outside_rth, "Stop loss should inherit outside_rth");
}

#[test]
fn test_bracket_order_market_entry_quantity_propagation() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(500)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();

    // All orders should have the same quantity
    assert_eq!(orders[0].total_quantity, 500.0);
    assert_eq!(orders[1].total_quantity, 500.0);
    assert_eq!(orders[2].total_quantity, 500.0);
}

#[test]
fn test_bracket_order_market_entry_parent_id_propagation() {
    let client = MockClient;
    let contract = create_test_contract();

    let bracket = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_market()
        .take_profit(55.0)
        .stop_loss(45.0);

    let orders = bracket.build().unwrap();
    let parent_id = orders[0].order_id;

    assert_eq!(orders[1].parent_id, parent_id);
    assert_eq!(orders[2].parent_id, parent_id);
}

#[test]
fn test_market_on_close() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_on_close();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MOC");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_limit_on_close() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_on_close(50.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LOC");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, Some(50.50));
}

#[test]
fn test_market_on_open() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_on_open();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MKT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.tif, TimeInForce::OnOpen);
    assert_eq!(order.limit_price, None);
}

#[test]
fn test_limit_on_open() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit_on_open(50.50);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, Some(50.50));
    assert_eq!(order.tif, TimeInForce::OnOpen);
}

#[test]
fn test_market_with_protection() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(100).market_with_protection();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MKT PRT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.limit_price, None);
    assert_eq!(order.aux_price, None);
}

#[test]
fn test_stop_with_protection() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(100).stop_with_protection(95.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "STP PRT");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.aux_price, Some(95.00));
}

#[test]
fn test_market_on_close_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(200).market_on_close();

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "MOC");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 200.0);
}

#[test]
fn test_limit_on_close_sell() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).sell(200).limit_on_close(100.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "LOC");
    assert_eq!(order.action, Action::Sell);
    assert_eq!(order.total_quantity, 200.0);
    assert_eq!(order.limit_price, Some(100.00));
}

#[test]
fn test_stop_with_protection_buy() {
    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract).buy(50).stop_with_protection(105.00);

    let order = builder.build().unwrap();
    assert_eq!(order.order_type, "STP PRT");
    assert_eq!(order.action, Action::Buy);
    assert_eq!(order.total_quantity, 50.0);
    assert_eq!(order.aux_price, Some(105.00));
}

// ===== Conditional Order Tests =====

#[test]
fn test_single_price_condition() {
    use crate::orders::builder::price;
    use crate::orders::OrderCondition;

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);

    match &order.conditions[0] {
        OrderCondition::Price(c) => {
            assert_eq!(c.contract_id, 265598);
            assert_eq!(c.exchange, "SMART");
            assert_eq!(c.price, 150.0);
            assert!(c.is_more);
            assert!(c.is_conjunction);
        }
        _ => panic!("Expected Price condition"),
    }
}

#[test]
fn test_multiple_and_conditions() {
    use crate::orders::builder::{margin, price, time};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0))
        .and_condition(margin().greater_than(30))
        .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 3);

    // All conditions should have is_conjunction = true for AND logic
    for cond in &order.conditions {
        match cond {
            crate::orders::OrderCondition::Price(c) => assert!(c.is_conjunction),
            crate::orders::OrderCondition::Margin(c) => assert!(c.is_conjunction),
            crate::orders::OrderCondition::Time(c) => assert!(c.is_conjunction),
            _ => {}
        }
    }
}

#[test]
fn test_multiple_or_conditions() {
    use crate::orders::builder::{price, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").less_than(100.0))
        .or_condition(volume(265598, "SMART").greater_than(50_000_000));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 2);

    // First condition should have is_conjunction = false for OR with next
    match &order.conditions[0] {
        crate::orders::OrderCondition::Price(c) => assert!(!c.is_conjunction),
        _ => panic!("Expected Price condition"),
    }
}

#[test]
fn test_or_condition_sets_conjunction_on_unknown() {
    use crate::orders::builder::price;
    use crate::orders::{OrderCondition, UnknownCondition};

    let client = MockClient;
    let contract = create_test_contract();
    let unknown = OrderCondition::Unknown(UnknownCondition {
        condition_type: 2,
        is_conjunction: true,
        ..Default::default()
    });

    let order = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(unknown)
        .or_condition(price(265598, "SMART").less_than(100.0))
        .build()
        .unwrap();

    assert!(!order.conditions[0].is_conjunction());
}

#[test]
fn test_mixed_and_or_conditions() {
    use crate::orders::builder::{margin, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    // (price > 10 AND margin < 20) OR time > X OR volume > Y
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(123445, "SMART").greater_than(10.0))
        .and_condition(margin().less_than(20))
        .or_condition(time().greater_than("20251010 09:30:00 US/Eastern"))
        .or_condition(volume(123445, "SMART").greater_than(10_000_000));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 4);

    // Check conjunction flags: AND, OR, OR pattern
    match &order.conditions[0] {
        crate::orders::OrderCondition::Price(c) => assert!(c.is_conjunction), // AND with next
        _ => panic!("Expected Price condition"),
    }
    match &order.conditions[1] {
        crate::orders::OrderCondition::Margin(c) => assert!(!c.is_conjunction), // OR with next
        _ => panic!("Expected Margin condition"),
    }
    match &order.conditions[2] {
        crate::orders::OrderCondition::Time(c) => assert!(!c.is_conjunction), // OR with next
        _ => panic!("Expected Time condition"),
    }
}

#[test]
fn test_all_condition_types() {
    use crate::orders::builder::{execution, margin, percent_change, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0))
        .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
        .and_condition(margin().less_than(30))
        .and_condition(execution("MSFT", "STK", "SMART"))
        .and_condition(volume(76792991, "SMART").greater_than(50_000_000))
        .and_condition(percent_change(756733, "SMART").greater_than(2.0));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 6);
}

#[test]
fn test_condition_builder_conversion() {
    use crate::orders::builder::price;

    let client = MockClient;
    let contract = create_test_contract();

    // Test that condition builder auto-converts to OrderCondition via Into
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0)); // Builder should auto-convert

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);
}

#[test]
fn test_condition_with_existing_order_conditions() {
    use crate::orders::builder::price;
    use crate::orders::conditions::MarginCondition;
    use crate::orders::OrderCondition;

    let client = MockClient;
    let contract = create_test_contract();

    // Test mixing fluent API with manual conditions
    let mut builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(price(265598, "SMART").greater_than(150.0));

    // Manually add another condition
    builder.conditions.push(OrderCondition::Margin(MarginCondition {
        percent: 25,
        is_more: false,
        is_conjunction: true,
    }));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 2);
}

#[test]
fn test_less_than_conditions() {
    use crate::orders::builder::{margin, percent_change, price, time, volume};

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .sell(100)
        .market()
        .condition(price(265598, "SMART").less_than(140.0))
        .and_condition(margin().less_than(25))
        .and_condition(volume(265598, "SMART").less_than(1_000_000))
        .and_condition(percent_change(265598, "SMART").less_than(-2.0))
        .and_condition(time().less_than("20251230 09:30:00 US/Eastern"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 5);

    // Verify all are less_than (is_more = false)
    for cond in &order.conditions {
        match cond {
            crate::orders::OrderCondition::Price(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Margin(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Volume(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::PercentChange(c) => assert!(!c.is_more),
            crate::orders::OrderCondition::Time(c) => assert!(!c.is_more),
            _ => {}
        }
    }
}

#[test]
fn test_execution_condition_no_threshold() {
    use crate::orders::builder::execution;

    let client = MockClient;
    let contract = create_test_contract();

    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .market()
        .condition(execution("TSLA", "STK", "SMART"));

    let order = builder.build().unwrap();
    assert_eq!(order.conditions.len(), 1);

    match &order.conditions[0] {
        crate::orders::OrderCondition::Execution(c) => {
            assert_eq!(c.symbol, "TSLA");
            assert_eq!(c.security_type, "STK");
            assert_eq!(c.exchange, "SMART");
        }
        _ => panic!("Expected Execution condition"),
    }
}

#[test]
fn sell_short_sets_action_and_quantity() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract).sell_short(100).limit(50.0).build().unwrap();

    assert_eq!(order.action, Action::SellShort);
    assert_eq!(order.action.to_string(), "SSHORT");
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.limit_price, Some(50.0));
}

#[test]
fn sell_long_sets_action_and_quantity() {
    let client = MockClient;
    let contract = create_test_contract();

    let order = OrderBuilder::new(&client, &contract).sell_long(75).market().build().unwrap();

    assert_eq!(order.action, Action::SellLong);
    assert_eq!(order.action.to_string(), "SLONG");
    assert_eq!(order.total_quantity, 75.0);
    assert_eq!(order.order_type, "MKT");
}

#[test]
fn bracket_order_propagates_tif() {
    let client = MockClient;
    let contract = create_test_contract();

    let orders = OrderBuilder::new(&client, &contract)
        .buy(100)
        .good_till_canceled()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0)
        .build()
        .unwrap();

    assert_eq!(orders[0].tif, TimeInForce::GoodTillCanceled);
    assert_eq!(orders[1].tif, TimeInForce::GoodTillCanceled);
    assert_eq!(orders[2].tif, TimeInForce::GoodTillCanceled);
}

fn counter(start: i32) -> impl FnMut() -> i32 {
    let mut next = start;
    move || {
        next += 1;
        next - 1
    }
}

#[test]
fn preset_legs_take_ids_after_the_parent() {
    let client = MockClient;
    let contract = create_test_contract();
    let base = || OrderBuilder::new(&client, &contract).buy(100).limit(50.0);

    let cases = [
        (base().preset_stop_loss(), Some(101), None),
        (base().preset_profit_taker(), None, Some(101)),
        (base().preset_stop_loss().preset_profit_taker(), Some(101), Some(102)),
        // Profit-taker first in the chain still numbers stop-loss first.
        (base().preset_profit_taker().preset_stop_loss(), Some(101), Some(102)),
        // Repeating a leg doesn't request it twice.
        (base().preset_stop_loss().preset_stop_loss(), Some(101), None),
    ];
    for (builder, stop_loss, profit_taker) in cases {
        let (order, ids) = builder.build_with_ids(counter(100)).expect("valid order");
        assert_eq!(order.order_id, 100);
        assert_eq!(order.preset_stop_loss_order_id, stop_loss);
        assert_eq!(order.preset_profit_taker_order_id, profit_taker);
        assert_eq!(ids.parent, OrderId(100));
        assert_eq!(ids.stop_loss, stop_loss.map(OrderId));
        assert_eq!(ids.profit_taker, profit_taker.map(OrderId));
    }
}

#[test]
fn preset_legs_consume_no_ids_for_an_invalid_parent() {
    let client = MockClient;
    let contract = create_test_contract();
    let mut calls = 0;

    let result = OrderBuilder::new(&client, &contract)
        .buy(-1)
        .limit(50.0)
        .preset_stop_loss()
        .build_with_ids(|| {
            calls += 1;
            calls
        });

    assert!(matches!(result, Err(ValidationError::InvalidQuantity(_))), "got {result:?}");
    assert_eq!(calls, 0);
}

// === Order::builder() (detached) ===

#[test]
fn detached_builder_matches_bound_builder() {
    let client = MockClient;
    let contract = create_test_contract();

    let bound = OrderBuilder::new(&client, &contract)
        .sell(100)
        .stop_limit(95.0, 94.5)
        .good_till_canceled()
        .outside_rth()
        .account("DU123")
        .build()
        .unwrap();
    let detached = Order::builder()
        .sell(100)
        .stop_limit(95.0, 94.5)
        .good_till_canceled()
        .outside_rth()
        .account("DU123")
        .build()
        .unwrap();

    assert_eq!(bound, detached);
}

#[test]
fn detached_builder_validates() {
    let err = Order::builder().buy(100).build().unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("order_type"));
}

// === Trailing stops: TrailBy ===

#[test]
fn trailing_stop_by_amount_sets_aux_price() {
    let order = Order::builder().sell(100).trailing_stop(TrailBy::Amount(2.0), 95.0).build().unwrap();
    assert_eq!(order.order_type, "TRAIL");
    assert_eq!(order.aux_price, Some(2.0));
    assert_eq!(order.trailing_percent, None);
    assert_eq!(order.trail_stop_price, Some(95.0));
}

#[test]
fn trailing_stop_by_percent_sets_trailing_percent() {
    let order = Order::builder().sell(100).trailing_stop(TrailBy::Percent(5.0), 95.0).build().unwrap();
    assert_eq!(order.trailing_percent, Some(5.0));
    assert_eq!(order.aux_price, None);
}

#[test]
fn trailing_stop_limit_by_amount() {
    let order = Order::builder()
        .buy(100)
        .trailing_stop_limit(TrailBy::Amount(1.0), 105.0, 0.25)
        .build()
        .unwrap();

    assert_eq!(order.order_type, "TRAIL LIMIT");
    assert_eq!(order.aux_price, Some(1.0));
    assert_eq!(order.limit_price_offset, Some(0.25));
    assert_eq!(order.trail_stop_price, Some(105.0));
    assert_eq!(order.trailing_percent, None);
}

#[test]
fn last_trail_wins() {
    let order = Order::builder()
        .sell(100)
        .trailing_stop(TrailBy::Percent(5.0), 95.0)
        .trailing_stop(TrailBy::Amount(2.0), 95.0)
        .build()
        .unwrap();
    assert_eq!(order.aux_price, Some(2.0));
    assert_eq!(order.trailing_percent, None);
}

#[test]
fn trail_amount_replaces_an_earlier_stop_price() {
    let order = Order::builder()
        .sell(100)
        .stop(100.0)
        .trailing_stop(TrailBy::Amount(2.0), 98.0)
        .build()
        .unwrap();
    assert_eq!(order.aux_price, Some(2.0));
    assert_eq!(order.trail_stop_price, Some(98.0));
}

#[test]
fn trail_percent_ignores_an_earlier_stop_price() {
    let order = Order::builder()
        .sell(100)
        .stop(100.0)
        .trailing_stop(TrailBy::Percent(5.0), 95.0)
        .build()
        .unwrap();
    assert_eq!(order.aux_price, None);
    assert_eq!(order.trailing_percent, Some(5.0));
}

#[test]
fn market_if_touched_ignores_an_earlier_stop_price() {
    let order = Order::builder().buy(100).stop(100.0).market_if_touched(105.0).build().unwrap();
    assert_eq!(order.aux_price, Some(105.0));
}

#[test]
fn stop_with_protection_requires_stop_price() {
    let err = Order::builder().buy(1).order_type(OrderType::StopWithProtection).build().unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("stop_price"));
}

#[test]
fn trailing_stop_without_trail_fails_validation() {
    let err = Order::builder().sell(100).order_type(OrderType::TrailingStop).build().unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("trailing amount or percent"));
}

// === Price fields from an earlier order-type setter ===

#[test]
fn limit_drops_an_earlier_trail_percent() {
    let order = Order::builder()
        .sell(100)
        .trailing_stop(TrailBy::Percent(5.0), 95.0)
        .limit(100.0)
        .build()
        .unwrap();
    assert_eq!(order.trailing_percent, None);
    assert_eq!(order.trail_stop_price, None);
}

#[test]
fn limit_drops_an_earlier_trail_amount() {
    let order = Order::builder()
        .sell(100)
        .trailing_stop(TrailBy::Amount(2.0), 95.0)
        .limit(100.0)
        .build()
        .unwrap();
    assert_eq!(order.aux_price, None);
}

#[test]
fn trailing_stop_limit_drops_an_earlier_limit_price() {
    let order = Order::builder()
        .sell(100)
        .stop_limit(95.0, 94.5)
        .trailing_stop_limit(TrailBy::Amount(2.0), 95.0, 0.5)
        .build()
        .unwrap();
    assert_eq!(order.limit_price, None);
    assert_eq!(order.limit_price_offset, Some(0.5));
}

#[test]
fn trailing_stop_drops_an_earlier_limit_offset() {
    let order = Order::builder()
        .sell(100)
        .trailing_stop_limit(TrailBy::Amount(2.0), 95.0, 0.5)
        .trailing_stop(TrailBy::Amount(2.0), 95.0)
        .build()
        .unwrap();
    assert_eq!(order.limit_price_offset, None);
}

#[test]
fn aux_types_require_aux_price() {
    for order_type in [
        OrderType::MarketIfTouched,
        OrderType::Relative,
        OrderType::PassiveRelative,
        OrderType::PeggedToMarket,
    ] {
        let err = Order::builder().buy(100).order_type(order_type.clone()).build().unwrap_err();
        assert_eq!(err, ValidationError::MissingRequiredField("aux_price"), "{order_type:?}");
    }

    let err = Order::builder()
        .buy(100)
        .limit(100.0)
        .order_type(OrderType::LimitIfTouched)
        .build()
        .unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("aux_price"));
}

#[test]
fn stop_price_does_not_stand_in_for_a_trigger() {
    let err = Order::builder()
        .buy(100)
        .stop(100.0)
        .order_type(OrderType::MarketIfTouched)
        .build()
        .unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("aux_price"));
}

#[test]
fn pegged_to_midpoint_type_does_not_require_aux() {
    let order = Order::builder()
        .buy(100)
        .limit(150.0)
        .order_type(OrderType::PeggedToMidpoint)
        .build()
        .unwrap();
    assert_eq!(order.aux_price, None);
    assert_eq!(order.limit_price, Some(150.0));
}

// === IBKRATS pegs ===

#[test]
fn peg_best_sends_the_c_sharp_shape() {
    let base = Order {
        action: Action::Buy,
        order_type: "PEG BEST".to_owned(),
        total_quantity: 100.0,
        limit_price: Some(111.11),
        not_held: true,
        min_trade_qty: Some(100),
        min_compete_size: Some(200),
        ..Order::default()
    };
    let cases = [
        (
            CompeteAgainstBest::Offset(0.03),
            Order {
                compete_against_best_offset: Some(0.03),
                ..base.clone()
            },
        ),
        (
            CompeteAgainstBest::UpToMid(UP_TO_MID),
            Order {
                compete_against_best_offset: Some(f64::INFINITY),
                mid_offset_at_whole: Some(0.02),
                mid_offset_at_half: Some(0.025),
                ..base.clone()
            },
        ),
    ];

    for (compete, expected) in cases {
        let order = Order::builder()
            .buy(100)
            .peg_best(111.11, compete)
            .min_trade_qty(100)
            .min_compete_size(200)
            .build()
            .unwrap();
        assert_eq!(order, expected, "{compete:?}");
    }
}

#[test]
fn peg_best_rejects_a_non_finite_offset() {
    for offset in [f64::INFINITY, f64::NAN] {
        let err = Order::builder()
            .buy(100)
            .peg_best(111.11, CompeteAgainstBest::Offset(offset))
            .build()
            .unwrap_err();
        assert!(matches!(err, ValidationError::InvalidPrice(_)), "{offset}");
    }
}

#[test]
fn peg_mid_sends_the_c_sharp_shape() {
    let order = Order::builder().buy(100).peg_mid(111.11, UP_TO_MID).build().unwrap();
    let expected = Order {
        action: Action::Buy,
        order_type: "PEG MID".to_owned(),
        total_quantity: 100.0,
        limit_price: Some(111.11),
        not_held: true,
        mid_offset_at_whole: Some(0.02),
        mid_offset_at_half: Some(0.025),
        ..Order::default()
    };
    assert_eq!(order, expected);
}

#[test]
fn peg_best_offset_drops_earlier_mid_offsets() {
    let up_to_mid = CompeteAgainstBest::UpToMid(UP_TO_MID);
    let order = Order::builder()
        .buy(100)
        .peg_best(111.11, up_to_mid)
        .peg_best(111.11, CompeteAgainstBest::Offset(0.03))
        .build()
        .unwrap();
    assert_eq!(order.mid_offset_at_whole, None);
    assert_eq!(order.mid_offset_at_half, None);
}

#[test]
fn other_types_drop_peg_best_fields() {
    let up_to_mid = CompeteAgainstBest::UpToMid(UP_TO_MID);
    let order = Order::builder()
        .buy(100)
        .peg_best(111.11, up_to_mid)
        .min_compete_size(200)
        .limit(100.0)
        .build()
        .unwrap();
    assert_eq!(order.compete_against_best_offset, None);
    assert_eq!(order.min_compete_size, None);
    assert_eq!(order.mid_offset_at_whole, None);
    assert_eq!(order.mid_offset_at_half, None);
}

#[test]
fn peg_mid_forms_replace_each_other() {
    let offset_form = Order::builder()
        .buy(100)
        .peg_mid(111.11, UP_TO_MID)
        .pegged_to_midpoint(0.01, 111.11)
        .build()
        .unwrap();
    assert_eq!(offset_form.aux_price, Some(0.01));
    assert_eq!(offset_form.mid_offset_at_whole, None);

    let ibkrats_form = Order::builder()
        .buy(100)
        .pegged_to_midpoint(0.01, 111.11)
        .peg_mid(111.11, UP_TO_MID)
        .build()
        .unwrap();
    assert_eq!(ibkrats_form.aux_price, None);
    assert_eq!(ibkrats_form.mid_offset_at_whole, Some(0.02));
}

// Every variant; add a new one here and in `sent_price_fields`.
const ALL_ORDER_TYPES: [OrderType; 30] = [
    OrderType::Market,
    OrderType::Limit,
    OrderType::Stop,
    OrderType::StopLimit,
    OrderType::TrailingStop,
    OrderType::TrailingStopLimit,
    OrderType::MarketOnClose,
    OrderType::LimitOnClose,
    OrderType::MarketOnOpen,
    OrderType::LimitOnOpen,
    OrderType::AtAuction,
    OrderType::MarketIfTouched,
    OrderType::LimitIfTouched,
    OrderType::MarketWithProtection,
    OrderType::StopWithProtection,
    OrderType::MarketToLimit,
    OrderType::Midprice,
    OrderType::PeggedToMarket,
    OrderType::PeggedToStock,
    OrderType::PeggedToMidpoint,
    OrderType::PeggedToBenchmark,
    OrderType::PegBest,
    OrderType::Relative,
    OrderType::PassiveRelative,
    OrderType::Volatility,
    OrderType::BoxTop,
    OrderType::ComboLimit,
    OrderType::ComboMarket,
    OrderType::RelativeLimitCombo,
    OrderType::RelativeMarketCombo,
];

/// A builder that sets every price field and every field a type requires, then switches to
/// `order_type`.
fn every_price_field(order_type: OrderType) -> OrderBuilder<Detached> {
    Order::builder()
        .buy(1)
        .peg_best(1.0, CompeteAgainstBest::UpToMid(UP_TO_MID))
        .min_compete_size(200)
        .peg_mid(1.0, PEG_MID)
        .trailing_stop_limit(TrailBy::Percent(5.0), 95.0, 0.5)
        .limit(100.0)
        .stop(99.0)
        .market_if_touched(98.0)
        .volatility(0.3)
        .pegged_to_stock(0.5, 2.0)
        .reference_contract(1, "ISLAND")
        .order_type(order_type)
}

const UP_TO_MID: MidOffsets = MidOffsets {
    at_whole: 0.02,
    at_half: 0.025,
};
const PEG_MID: MidOffsets = MidOffsets {
    at_whole: 0.03,
    at_half: 0.035,
};

/// Which price fields each type sends: (limit_price, aux_price, trail fields), per C#
/// `OrderSamples.cs` plus `OrderType::uses_*` for PEG BEST and the REL combos. Exhaustive, so a
/// new `OrderType` variant needs a row.
fn sent_price_fields(order_type: &OrderType) -> (bool, bool, bool) {
    use OrderType::*;
    match order_type {
        Limit | LimitOnClose | LimitOnOpen | AtAuction | Midprice | PegBest | ComboLimit => (true, false, false),
        StopLimit | LimitIfTouched | Relative | PeggedToMidpoint | RelativeLimitCombo => (true, true, false),
        Stop | StopWithProtection | MarketIfTouched | PassiveRelative | PeggedToMarket | RelativeMarketCombo => (false, true, false),
        TrailingStop | TrailingStopLimit => (false, true, true),
        Market | MarketOnClose | MarketOnOpen | MarketWithProtection | MarketToLimit | PeggedToStock | PeggedToBenchmark | Volatility | BoxTop
        | ComboMarket => (false, false, false),
    }
}

#[test]
fn each_type_sends_exactly_the_price_fields_it_uses() {
    for order_type in ALL_ORDER_TYPES {
        let order = every_price_field(order_type.clone()).build().unwrap();
        let (limit, aux, trail) = sent_price_fields(&order_type);
        // Stop types send `.stop(99.0)` as aux; the rest send `.market_if_touched(98.0)`'s trigger.
        let expected_aux = if order_type.uses_stop_price() { 99.0 } else { 98.0 };

        assert_eq!(order.limit_price, limit.then_some(100.0), "{order_type:?}");
        assert_eq!(order.aux_price, aux.then_some(expected_aux), "{order_type:?}");
        assert_eq!(order.trailing_percent, trail.then_some(5.0), "{order_type:?}");
        assert_eq!(order.trail_stop_price, trail.then_some(95.0), "{order_type:?}");
        assert_eq!(
            order.limit_price_offset,
            (order_type == OrderType::TrailingStopLimit).then_some(0.5),
            "{order_type:?}"
        );

        // IBKRATS fields: compete fields on PEG BEST only; mid offsets on PEG MID (from
        // `peg_mid`) and on PEG BEST up to the midpoint.
        let peg_best = order_type == OrderType::PegBest;
        let mid_offsets = match order_type {
            OrderType::PegBest => Some(UP_TO_MID),
            OrderType::PeggedToMidpoint => Some(PEG_MID),
            _ => None,
        };
        assert_eq!(order.min_compete_size, peg_best.then_some(200), "{order_type:?}");
        assert_eq!(order.compete_against_best_offset, peg_best.then_some(f64::INFINITY), "{order_type:?}");
        assert_eq!(order.mid_offset_at_whole, mid_offsets.map(|m| m.at_whole), "{order_type:?}");
        assert_eq!(order.mid_offset_at_half, mid_offsets.map(|m| m.at_half), "{order_type:?}");
    }
}

// === Pegged orders ===

#[test]
fn pegged_to_market_sets_offset() {
    let order = Order::builder().buy(100).pegged_to_market(0.05).build().unwrap();
    assert_eq!(order, crate::orders::order_builder::pegged_to_market(Action::Buy, 100.0, 0.05));
}

#[test]
fn pegged_to_midpoint_matches_free_fn_argument_order() {
    let order = Order::builder().buy(100).pegged_to_midpoint(0.01, 150.0).build().unwrap();
    assert_eq!(order, crate::orders::order_builder::pegged_to_midpoint(Action::Buy, 100.0, 0.01, 150.0));
}

#[test]
fn box_top_sets_order_type() {
    let order = Order::builder().buy(10).box_top().build().unwrap();
    assert_eq!(order, crate::orders::order_builder::box_top(Action::Buy, 10.0));
}

#[test]
fn pegged_to_stock_with_reference_price() {
    let order = Order::builder()
        .buy(1)
        .pegged_to_stock(0.5, 2.10)
        .stock_reference_price(150.0)
        .build()
        .unwrap();
    let expected = Order {
        action: Action::Buy,
        order_type: "PEG STK".to_owned(),
        total_quantity: 1.0,
        delta: Some(0.5),
        stock_ref_price: Some(150.0),
        starting_price: Some(2.10),
        ..Order::default()
    };
    assert_eq!(order, expected);
}

#[test]
fn pegged_to_stock_without_reference_price() {
    let order = Order::builder().buy(1).pegged_to_stock(0.5, 2.10).build().unwrap();
    let expected = Order {
        action: Action::Buy,
        order_type: "PEG STK".to_owned(),
        total_quantity: 1.0,
        delta: Some(0.5),
        starting_price: Some(2.10),
        ..Order::default()
    };
    assert_eq!(order, expected);
}

#[test]
fn stock_range_sets_bounds() {
    let order = Order::builder()
        .buy(1)
        .pegged_to_stock(0.5, 2.10)
        .stock_range(140.0, 160.0)
        .build()
        .unwrap();
    assert_eq!(order.stock_range_lower, Some(140.0));
    assert_eq!(order.stock_range_upper, Some(160.0));
}

#[test]
fn pegged_to_stock_requires_delta() {
    let err = Order::builder().buy(1).order_type(OrderType::PeggedToStock).build().unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("delta"));
}

#[test]
fn pegged_to_stock_requires_starting_price() {
    let mut builder = Order::builder().buy(1).order_type(OrderType::PeggedToStock);
    builder.delta = Some(0.5);
    assert_eq!(builder.build().unwrap_err(), ValidationError::MissingRequiredField("starting_price"));
}

#[test]
fn pegged_to_benchmark_sets_every_field() {
    let order = Order::builder()
        .buy(100)
        .pegged_to_benchmark(50.0)
        .reference_contract(12345, "ISLAND")
        .pegged_change_amount(0.02)
        .pegged_change_amount_decrease()
        .reference_change_amount(0.01)
        .stock_reference_price(49.0)
        .reference_range(48.0, 52.0)
        .build()
        .unwrap();
    let expected = Order {
        action: Action::Buy,
        order_type: "PEG BENCH".to_owned(),
        total_quantity: 100.0,
        starting_price: Some(50.0),
        is_pegged_change_amount_decrease: true,
        pegged_change_amount: Some(0.02),
        reference_change_amount: Some(0.01),
        reference_contract_id: 12345,
        reference_exchange: "ISLAND".to_owned(),
        stock_ref_price: Some(49.0),
        stock_range_lower: Some(48.0),
        stock_range_upper: Some(52.0),
        ..Order::default()
    };
    assert_eq!(order, expected);
}

#[test]
fn reference_contract_accepts_contract_id() {
    use crate::accounts::types::ContractId;
    let order = Order::builder()
        .buy(100)
        .pegged_to_benchmark(50.0)
        .reference_contract(ContractId(12345), "ISLAND")
        .build()
        .unwrap();
    assert_eq!(order.reference_contract_id, 12345);
    assert_eq!(order.reference_exchange, "ISLAND");
}

#[test]
fn pegged_to_benchmark_requires_reference_contract() {
    let err = Order::builder().buy(100).pegged_to_benchmark(50.0).build().unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("reference_contract"));
}

#[test]
fn pegged_to_benchmark_requires_starting_price() {
    let err = Order::builder()
        .buy(100)
        .order_type(OrderType::PeggedToBenchmark)
        .reference_contract(12345, "ISLAND")
        .build()
        .unwrap_err();
    assert_eq!(err, ValidationError::MissingRequiredField("starting_price"));
}

// === Combo, manual time, cash quantity ===

#[test]
fn non_guaranteed_matches_free_fn() {
    let order = Order::builder().buy(1).limit(2.5).non_guaranteed().build().unwrap();
    let free = crate::orders::order_builder::non_guaranteed(crate::orders::order_builder::combo_limit_order(Action::Buy, 1.0, 2.5));
    assert_eq!(order.smart_combo_routing_params, free.smart_combo_routing_params);
}

#[test]
fn combo_leg_prices_set_one_leg_per_price() {
    let order = Order::builder().buy(1).limit(2.5).combo_leg_prices([1.1, 2.2]).build().unwrap();
    let free = crate::orders::order_builder::limit_order_for_combo_with_leg_prices(Action::Buy, 1.0, vec![1.1, 2.2]);
    assert_eq!(order.order_combo_legs, free.order_combo_legs);
}

#[test]
fn manual_order_time_sets_field() {
    let order = Order::builder()
        .buy(100)
        .limit(50.0)
        .manual_order_time("20260103 10:00:00")
        .build()
        .unwrap();
    assert_eq!(order.manual_order_time, "20260103 10:00:00");
}

#[test]
fn cash_qty_allows_zero_quantity() {
    let order = Order::builder().buy(0).limit(1.10).cash_qty(20_000.0).build().unwrap();
    assert_eq!(
        order,
        crate::orders::order_builder::limit_order_with_cash_qty(Action::Buy, 1.10, 20_000.0)
    );
}

#[test]
fn cash_qty_keeps_a_nonzero_quantity() {
    let order = Order::builder().buy(100).limit(1.10).cash_qty(20_000.0).build().unwrap();
    assert_eq!(order.total_quantity, 100.0);
}

#[test]
fn cash_qty_must_be_positive() {
    let err = Order::builder().buy(0).limit(1.10).cash_qty(0.0).build().unwrap_err();
    assert_eq!(err, ValidationError::InvalidQuantity(0.0));
}

#[test]
fn zero_quantity_without_cash_qty_still_fails() {
    let err = Order::builder().buy(0).limit(1.10).build().unwrap_err();
    assert_eq!(err, ValidationError::InvalidQuantity(0.0));
}
