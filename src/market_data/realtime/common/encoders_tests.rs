use super::*;
use crate::common::test_utils::helpers::assert_proto_msg_id;

#[test]
fn encode_request_market_data_type_round_trip() {
    let bytes = encode_request_market_data_type(MarketDataType::Delayed).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::RequestMarketDataType);

    use prost::Message;
    let req = crate::proto::MarketDataTypeRequest::decode(&bytes[4..]).unwrap();
    assert_eq!(req.market_data_type, Some(3));
}
