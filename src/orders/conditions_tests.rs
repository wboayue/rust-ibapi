use super::*;

#[test]
fn price_condition_builder() {
    let condition = PriceCondition::builder(12345, "NASDAQ")
        .greater_than(150.0)
        .trigger_method(TriggerMethod::DoubleBidAsk)
        .conjunction(false)
        .build();

    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "NASDAQ");
    assert_eq!(condition.price, 150.0);
    assert_eq!(condition.trigger_method, TriggerMethod::DoubleBidAsk);
    assert!(condition.is_more);
    assert!(!condition.is_conjunction);
}

/// Setters called before the comparison survive the move to the complete state.
#[test]
fn setters_before_the_comparison_carry_over() {
    let price = PriceCondition::builder(12345, "NASDAQ")
        .trigger_method(TriggerMethod::Last)
        .conjunction(false)
        .less_than(150.0)
        .build();
    assert_eq!(
        (
            price.contract_id,
            price.exchange.as_str(),
            price.trigger_method,
            price.is_more,
            price.is_conjunction
        ),
        (12345, "NASDAQ", TriggerMethod::Last, false, false)
    );

    let time = TimeCondition::builder().conjunction(false).greater_than("20251230 23:59:59 UTC").build();
    assert_eq!((time.is_more, time.is_conjunction), (true, false));

    let margin = MarginCondition::builder().conjunction(false).greater_than(30).build();
    assert_eq!((margin.is_more, margin.is_conjunction), (true, false));

    let volume = VolumeCondition::builder(12345, "NASDAQ").conjunction(false).greater_than(1000).build();
    assert_eq!(
        (volume.contract_id, volume.exchange.as_str(), volume.is_more, volume.is_conjunction),
        (12345, "NASDAQ", true, false)
    );

    let percent = PercentChangeCondition::builder(12345, "NASDAQ").conjunction(false).less_than(5.0).build();
    assert_eq!(
        (percent.contract_id, percent.exchange.as_str(), percent.is_more, percent.is_conjunction),
        (12345, "NASDAQ", false, false)
    );
}

#[test]
fn time_condition_builder() {
    let condition = TimeCondition::builder().less_than("20251230 23:59:59 UTC").build();

    assert_eq!(condition.time, "20251230 23:59:59 UTC");
    assert!(!condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
fn margin_condition_builder() {
    let condition = MarginCondition::builder().less_than(30).conjunction(false).build();

    assert_eq!(condition.percent, 30);
    assert!(!condition.is_more);
    assert!(!condition.is_conjunction);
}

#[test]
fn execution_condition_builder() {
    let condition = ExecutionCondition::builder("AAPL", "STK", "SMART").conjunction(false).build();

    assert_eq!(condition.symbol, "AAPL");
    assert_eq!(condition.security_type, "STK");
    assert_eq!(condition.exchange, "SMART");
    assert!(!condition.is_conjunction);
}

#[test]
fn volume_condition_builder() {
    let condition = VolumeCondition::builder(12345, "NASDAQ").less_than(1000000).build();

    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "NASDAQ");
    assert_eq!(condition.volume, 1000000);
    assert!(!condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
fn percent_change_condition_builder() {
    let condition = PercentChangeCondition::builder(12345, "NASDAQ")
        .greater_than(5.0)
        .conjunction(false)
        .build();

    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "NASDAQ");
    assert_eq!(condition.percent, 5.0);
    assert!(condition.is_more);
    assert!(!condition.is_conjunction);
}

#[test]
fn default_values() {
    let condition = PriceCondition::builder(12345, "NASDAQ").greater_than(150.0).build();

    assert_eq!(condition.trigger_method, TriggerMethod::Default);
    assert!(condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
fn trigger_method_round_trips_every_wire_code() {
    crate::common::test_utils::wire_enum::check_wire_code_round_trip(
        &[
            (TriggerMethod::Default, 0),
            (TriggerMethod::DoubleBidAsk, 1),
            (TriggerMethod::Last, 2),
            (TriggerMethod::DoubleLast, 3),
            (TriggerMethod::BidAsk, 4),
            (TriggerMethod::LastOrBidAsk, 7),
            (TriggerMethod::Midpoint, 8),
            // 5 and 6 are unassigned codes; neither may collapse to Default.
            (TriggerMethod::Unknown(5), 5),
            (TriggerMethod::Unknown(6), 6),
            (TriggerMethod::Unknown(9), 9),
        ],
        TriggerMethod::Unknown,
    );
}

#[test]
fn unknown_condition_accessors_and_serde() {
    use crate::orders::OrderCondition;

    let condition = OrderCondition::Unknown(UnknownCondition {
        condition_type: 2,
        is_conjunction: false,
        contract_id: Some(265598),
        time: Some("20251230 14:30:00 US/Eastern".into()),
        ..Default::default()
    });
    assert_eq!(condition.condition_type(), 2);
    assert!(!condition.is_conjunction());

    let json = serde_json::to_string(&condition).unwrap();
    assert!(json.starts_with("{\"Unknown\":"), "got {json}");
    assert_eq!(serde_json::from_str::<OrderCondition>(&json).unwrap(), condition);
}
