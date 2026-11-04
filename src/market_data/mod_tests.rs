use super::*;

#[test]
fn trading_hours_use_rth() {
    assert!(TradingHours::Regular.use_rth());
    assert!(!TradingHours::Extended.use_rth());
}

#[test]
fn trading_hours_from_use_rth() {
    assert_eq!(TradingHours::from_use_rth(true), TradingHours::Regular);
    assert_eq!(TradingHours::from_use_rth(false), TradingHours::Extended);
}

#[test]
fn trading_hours_default() {
    assert_eq!(TradingHours::default(), TradingHours::Regular);
}

#[test]
fn market_data_type_round_trips_every_wire_code() {
    crate::common::test_utils::wire_enum::check_wire_code_round_trip(
        &[
            (MarketDataType::Realtime, 1),
            (MarketDataType::Frozen, 2),
            (MarketDataType::Delayed, 3),
            (MarketDataType::DelayedFrozen, 4),
            (MarketDataType::Unknown(0), 0),
            (MarketDataType::Unknown(9), 9),
        ],
        MarketDataType::Unknown,
    );
}

#[test]
fn smart_depth_default_is_no() {
    assert_eq!(SmartDepth::default(), SmartDepth::No);
}
