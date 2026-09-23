use super::*;

#[test]
fn test_price_condition_builder() {
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

#[test]
fn test_time_condition_builder() {
    let condition = TimeCondition::builder().less_than("20251230 23:59:59 UTC").build();

    assert_eq!(condition.time, "20251230 23:59:59 UTC");
    assert!(!condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
fn test_margin_condition_builder() {
    let condition = MarginCondition::builder().less_than(30).conjunction(false).build();

    assert_eq!(condition.percent, 30);
    assert!(!condition.is_more);
    assert!(!condition.is_conjunction);
}

#[test]
fn test_execution_condition_builder() {
    let condition = ExecutionCondition::builder("AAPL", "STK", "SMART").conjunction(false).build();

    assert_eq!(condition.symbol, "AAPL");
    assert_eq!(condition.security_type, "STK");
    assert_eq!(condition.exchange, "SMART");
    assert!(!condition.is_conjunction);
}

#[test]
fn test_volume_condition_builder() {
    let condition = VolumeCondition::builder(12345, "NASDAQ").less_than(1000000).build();

    assert_eq!(condition.contract_id, 12345);
    assert_eq!(condition.exchange, "NASDAQ");
    assert_eq!(condition.volume, 1000000);
    assert!(!condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
fn test_percent_change_condition_builder() {
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
fn test_default_values() {
    let condition = PriceCondition::builder(12345, "NASDAQ").greater_than(150.0).build();

    assert_eq!(condition.trigger_method, TriggerMethod::Default);
    assert!(condition.is_more);
    assert!(condition.is_conjunction);
}

#[test]
#[should_panic(expected = "PriceConditionBuilder requires a price threshold")]
fn test_price_condition_builder_missing_threshold_panics() {
    let _ = PriceCondition::builder(12345, "NASDAQ").build();
}

#[test]
#[should_panic(expected = "TimeConditionBuilder requires a time value")]
fn test_time_condition_builder_missing_time_panics() {
    let _ = TimeCondition::builder().build();
}

#[test]
#[should_panic(expected = "MarginConditionBuilder requires a percentage threshold")]
fn test_margin_condition_builder_missing_threshold_panics() {
    let _ = MarginCondition::builder().build();
}

#[test]
#[should_panic(expected = "VolumeConditionBuilder requires a volume threshold")]
fn test_volume_condition_builder_missing_threshold_panics() {
    let _ = VolumeCondition::builder(12345, "NASDAQ").build();
}

#[test]
#[should_panic(expected = "PercentChangeConditionBuilder requires a threshold")]
fn test_percent_change_condition_builder_missing_threshold_panics() {
    let _ = PercentChangeCondition::builder(12345, "NASDAQ").build();
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
