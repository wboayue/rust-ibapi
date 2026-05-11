use super::*;
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::messages::OutgoingMessages;

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
fn market_data_type_from_i32() {
    assert_eq!(MarketDataType::from(1), MarketDataType::Realtime);
    assert_eq!(MarketDataType::from(2), MarketDataType::Frozen);
    assert_eq!(MarketDataType::from(3), MarketDataType::Delayed);
    assert_eq!(MarketDataType::from(4), MarketDataType::DelayedFrozen);
    assert_eq!(MarketDataType::from(0), MarketDataType::Unknown);
}

#[test]
fn encode_request_market_data_type_round_trip() {
    let bytes = encoders::encode_request_market_data_type(MarketDataType::Delayed).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestMarketDataType);

    use prost::Message;
    let req = crate::proto::MarketDataTypeRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.market_data_type, Some(3));
}
