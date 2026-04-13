use crate::contracts::{Contract, TagValue};
use crate::market_data::realtime::WhatToShow;
use crate::messages::OutgoingMessages;
use crate::Error;

pub(crate) fn encode_request_realtime_bars(
    request_id: i32,
    contract: &Contract,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: &[TagValue],
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::RealTimeBarsRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        bar_size: Some(0),
        what_to_show: Some(what_to_show.to_string()),
        use_rth: some_bool(use_rth),
        real_time_bars_options: crate::proto::encoders::tag_values_to_map(options),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestRealTimeBars as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_realtime_bars(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelRealTimeBars, OutgoingMessages::CancelRealTimeBars)
}

pub(crate) fn encode_tick_by_tick(
    request_id: i32,
    contract: &Contract,
    tick_type: &str,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    let request = crate::proto::TickByTickRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        tick_type: some_str(tick_type),
        number_of_ticks: Some(number_of_ticks),
        ignore_size: some_bool(ignore_size),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestTickByTickData as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_tick_by_tick(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelTickByTick, OutgoingMessages::CancelTickByTickData)
}

pub(crate) fn encode_request_market_depth(request_id: i32, contract: &Contract, number_of_rows: i32, is_smart_depth: bool) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::MarketDepthRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        num_rows: Some(number_of_rows),
        is_smart_depth: some_bool(is_smart_depth),
        market_depth_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestMarketDepth as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_market_depth(request_id: i32, is_smart_depth: bool) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::CancelMarketDepth {
        req_id: Some(request_id),
        is_smart_depth: some_bool(is_smart_depth),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::CancelMarketDepth as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_request_market_depth_exchanges() -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::MarketDepthExchangesRequest {};
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestMktDepthExchanges as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_request_market_data(
    request_id: i32,
    contract: &Contract,
    generic_ticks: &[&str],
    snapshot: bool,
    regulatory_snapshot: bool,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    let joined = generic_ticks.join(",");
    let request = crate::proto::MarketDataRequest {
        req_id: Some(request_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        generic_tick_list: some_str(&joined),
        snapshot: some_bool(snapshot),
        regulatory_snapshot: some_bool(regulatory_snapshot),
        market_data_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestMarketData as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_market_data(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelMarketData, OutgoingMessages::CancelMarketData)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::Contract;

    fn create_test_contract() -> Contract {
        Contract::stock("AAPL").build()
    }

    #[test]
    fn test_encode_request_market_data() {
        let contract = create_test_contract();
        let bytes = encode_request_market_data(9000, &contract, &["100", "101"], false, false).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestMarketData);

        use prost::Message;
        let req = crate::proto::MarketDataRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.generic_tick_list.as_deref(), Some("100,101"));
        assert_eq!(req.contract.unwrap().symbol.as_deref(), Some("AAPL"));
        assert!(req.snapshot.is_none());
        assert!(req.regulatory_snapshot.is_none());
    }

    #[test]
    fn test_encode_cancel_market_data() {
        let bytes = encode_cancel_market_data(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelMarketData);
    }

    #[test]
    fn test_encode_tick_by_tick() {
        let contract = create_test_contract();
        let bytes = encode_tick_by_tick(9000, &contract, "AllLast", 1, true).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestTickByTickData);

        use prost::Message;
        let req = crate::proto::TickByTickRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.tick_type.as_deref(), Some("AllLast"));
        assert_eq!(req.number_of_ticks, Some(1));
        assert_eq!(req.ignore_size, Some(true));
    }

    #[test]
    fn test_encode_cancel_tick_by_tick() {
        let bytes = encode_cancel_tick_by_tick(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelTickByTickData);
    }

    #[test]
    fn test_encode_request_realtime_bars() {
        let contract = create_test_contract();
        let bytes = encode_request_realtime_bars(9000, &contract, &WhatToShow::Trades, true, &[]).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestRealTimeBars);
    }

    #[test]
    fn test_encode_cancel_realtime_bars() {
        let bytes = encode_cancel_realtime_bars(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelRealTimeBars);
    }

    #[test]
    fn test_encode_request_market_depth() {
        let contract = create_test_contract();
        let bytes = encode_request_market_depth(9000, &contract, 5, true).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestMarketDepth);

        use prost::Message;
        let req = crate::proto::MarketDepthRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.num_rows, Some(5));
        assert_eq!(req.is_smart_depth, Some(true));
    }

    #[test]
    fn test_encode_cancel_market_depth() {
        let bytes = encode_cancel_market_depth(9000, true).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelMarketDepth);

        use prost::Message;
        let req = crate::proto::CancelMarketDepth::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(9000));
        assert_eq!(req.is_smart_depth, Some(true));
    }

    #[test]
    fn test_encode_request_market_depth_exchanges() {
        let bytes = encode_request_market_depth_exchanges().unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestMktDepthExchanges);
    }
}
