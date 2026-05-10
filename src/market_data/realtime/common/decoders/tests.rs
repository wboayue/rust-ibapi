use super::*;
use crate::messages::ResponseMessage;
use crate::subscriptions::DecoderContext;
use crate::testdata::builders::market_data::{
    bid_ask_tick, market_depth_response, mid_point_tick, realtime_bar_tick, tick_generic, tick_price, tick_size, tick_string, trade_tick,
    BidAskTickResponse, MidPointTickResponse, TickGenericResponse, TickPriceResponse, TickSizeResponse, TickStringResponse, TradeTickResponse,
};
use crate::testdata::builders::ResponseProtoEncoder;
use time::OffsetDateTime;

mod realtime_bar_tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        realtime_bar_tick()
            .time(1678323335)
            .ohlc(4028.75, 4029.00, 4028.25, 4028.50)
            .volume(2.0)
            .wap(4026.75)
            .count(1)
            .encode_proto()
    }

    #[test]
    fn test_decode_realtime_bar_proto() {
        let bar = decode_realtime_bar_proto(&fixture()).expect("decode failed");

        assert_eq!(bar.date, OffsetDateTime::from_unix_timestamp(1678323335).unwrap());
        assert_eq!(bar.open, 4028.75);
        assert_eq!(bar.high, 4029.00);
        assert_eq!(bar.low, 4028.25);
        assert_eq!(bar.close, 4028.50);
        assert_eq!(bar.volume, 2.0);
        assert_eq!(bar.wap, 4026.75);
        assert_eq!(bar.count, 1);
    }

    #[test]
    fn test_decode_realtime_bar_through_wrapper() {
        let mut message = ResponseMessage::from_protobuf(
            crate::messages::IncomingMessages::RealTimeBars as i32,
            fixture(),
            server_versions::PROTOBUF_HISTORICAL_DATA,
        );
        let bar = decode_realtime_bar(&mut message).expect("decode failed");
        assert_eq!(bar.open, 4028.75);
    }

    #[test]
    fn test_decode_realtime_bar_invalid_proto_bytes() {
        let result = decode_realtime_bar_proto(&[0xff, 0xff, 0xff]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_realtime_bar_text_arrival_skip_classifies() {
        // Text arrival at a proto-only decoder must surface UnexpectedResponse so
        // the dispatcher skip-classifies (rule 20) rather than terminating.
        let mut message = ResponseMessage::from("50\0\09000\01678323335\04028.75\04029.00\04028.25\04028.50\02\04026.75\01\0");
        match decode_realtime_bar(&mut message) {
            Err(Error::UnexpectedResponse(_)) => {}
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }
}

mod trade_tick_tests {
    use super::*;

    fn fixture(tick_type: i32) -> TradeTickResponse {
        trade_tick()
            .tick_type(tick_type)
            .time(1678740829)
            .price(3895.25)
            .size(7.0)
            .attributes(false, true)
            .exchange("NASDAQ")
            .special_conditions("Regular")
    }

    #[test]
    fn test_decode_trade_tick_proto_last() {
        let trade = decode_trade_tick_proto(&fixture(1).encode_proto()).expect("decode failed");
        assert_eq!(trade.tick_type, "1");
        assert_eq!(trade.time, OffsetDateTime::from_unix_timestamp(1678740829).unwrap());
        assert_eq!(trade.price, 3895.25);
        assert_eq!(trade.size, 7.0);
        assert!(!trade.trade_attribute.past_limit);
        assert!(trade.trade_attribute.unreported);
        assert_eq!(trade.exchange, "NASDAQ");
        assert_eq!(trade.special_conditions, "Regular");
    }

    #[test]
    fn test_decode_trade_tick_proto_all_last() {
        let trade = decode_trade_tick_proto(&fixture(2).encode_proto()).expect("decode failed");
        assert_eq!(trade.tick_type, "2");
    }

    #[test]
    fn test_decode_trade_tick_proto_invalid_type() {
        // tick_type 3 = BidAsk — wrong feed for the trade decoder.
        let err = decode_trade_tick_proto(&fixture(3).encode_proto()).expect_err("should reject bid/ask tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }

    #[test]
    fn test_decode_trade_tick_proto_missing_payload() {
        // Build a tick_type=1 envelope with no inner Tick variant.
        let msg = crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(1),
            tick: None,
        };
        let result = decode_trade_tick_proto(&msg.encode_to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_trade_tick_through_wrapper() {
        let mut message = ResponseMessage::from_protobuf(
            crate::messages::IncomingMessages::TickByTick as i32,
            fixture(1).encode_proto(),
            server_versions::PROTOBUF_HISTORICAL_DATA,
        );
        let trade = decode_trade_tick(&mut message).expect("decode failed");
        assert_eq!(trade.price, 3895.25);
    }
}

mod bid_ask_tests {
    use super::*;

    fn fixture(mask: u32) -> BidAskTickResponse {
        bid_ask_tick()
            .time(1678745793)
            .quote(3895.50, 3896.00, 9.0, 11.0)
            .attributes(mask & 0x1 != 0, mask & 0x2 != 0)
    }

    #[test]
    fn test_decode_bid_ask_proto_basic() {
        let bid_ask = decode_bid_ask_tick_proto(&fixture(3).encode_proto()).expect("decode failed");
        assert_eq!(bid_ask.time, OffsetDateTime::from_unix_timestamp(1678745793).unwrap());
        assert_eq!(bid_ask.bid_price, 3895.50);
        assert_eq!(bid_ask.ask_price, 3896.00);
        assert_eq!(bid_ask.bid_size, 9.0);
        assert_eq!(bid_ask.ask_size, 11.0);
        assert!(bid_ask.bid_ask_attribute.bid_past_low);
        assert!(bid_ask.bid_ask_attribute.ask_past_high);
    }

    #[test]
    fn test_decode_bid_ask_proto_attributes() {
        for (mask, expected_bid_past_low, expected_ask_past_high) in [(0, false, false), (1, true, false), (2, false, true), (3, true, true)] {
            let bid_ask = decode_bid_ask_tick_proto(&fixture(mask).encode_proto()).expect("decode failed");
            assert_eq!(bid_ask.bid_ask_attribute.bid_past_low, expected_bid_past_low, "mask {mask}");
            assert_eq!(bid_ask.bid_ask_attribute.ask_past_high, expected_ask_past_high, "mask {mask}");
        }
    }

    #[test]
    fn test_decode_bid_ask_proto_invalid_type() {
        // tick_type 1 = Last — wrong feed.
        let msg = crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(1),
            tick: None,
        };
        let err = decode_bid_ask_tick_proto(&msg.encode_to_vec()).expect_err("should reject last tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }
}

mod mid_point_tests {
    use super::*;

    fn fixture() -> MidPointTickResponse {
        mid_point_tick().time(1678740829).mid_point(3895.375)
    }

    #[test]
    fn test_decode_mid_point_tick_proto() {
        let mid = decode_mid_point_tick_proto(&fixture().encode_proto()).expect("decode failed");
        assert_eq!(mid.time, OffsetDateTime::from_unix_timestamp(1678740829).unwrap());
        assert_eq!(mid.mid_point, 3895.375);
    }

    #[test]
    fn test_decode_mid_point_tick_proto_invalid_type() {
        let msg = crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(1),
            tick: None,
        };
        let err = decode_mid_point_tick_proto(&msg.encode_to_vec()).expect_err("should reject last tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }
}

mod market_depth_tests {
    use super::*;

    #[test]
    fn test_decode_market_depth_proto_basic() {
        let bytes = market_depth_response()
            .position(0)
            .operation(1)
            .side(1)
            .price(185.50)
            .size(100.0)
            .encode_proto();
        let depth = decode_market_depth_proto(&bytes).expect("decode failed");
        assert_eq!(depth.position, 0);
        assert_eq!(depth.operation, 1);
        assert_eq!(depth.side, 1);
        assert_eq!(depth.price, 185.50);
        assert_eq!(depth.size, 100.0);
    }

    #[test]
    fn test_decode_market_depth_proto_operations() {
        for op in [0, 1, 2] {
            let bytes = market_depth_response().operation(op).side(1).price(185.50).size(100.0).encode_proto();
            let depth = decode_market_depth_proto(&bytes).expect("decode failed");
            assert_eq!(depth.operation, op);
        }
    }

    #[test]
    fn test_decode_market_depth_proto_sides() {
        for side in [0, 1] {
            let bytes = market_depth_response().side(side).price(185.50).size(100.0).encode_proto();
            let depth = decode_market_depth_proto(&bytes).expect("decode failed");
            assert_eq!(depth.side, side);
        }
    }

    #[test]
    fn test_decode_market_depth_proto_missing_data() {
        let msg = crate::proto::MarketDepth {
            req_id: Some(9000),
            market_depth_data: None,
        };
        let err = decode_market_depth_proto(&msg.encode_to_vec()).expect_err("missing data should error");
        assert!(err.to_string().contains("missing market_depth_data"));
    }

    #[test]
    fn test_decode_market_depth_l2_proto() {
        // L2 carries market_maker + is_smart_depth, fields the MarketDepthResponse builder
        // doesn't expose; build the proto directly.
        let proto_msg = crate::proto::MarketDepthL2 {
            req_id: Some(9000),
            market_depth_data: Some(crate::proto::MarketDepthData {
                position: Some(0),
                operation: Some(1),
                side: Some(1),
                price: Some(185.50),
                size: Some("100".into()),
                market_maker: Some("ISLAND".into()),
                is_smart_depth: Some(true),
            }),
        };
        let depth = decode_market_depth_l2_proto(&proto_msg.encode_to_vec()).expect("decode failed");

        assert_eq!(depth.position, 0);
        assert_eq!(depth.market_maker, "ISLAND");
        assert_eq!(depth.operation, 1);
        assert_eq!(depth.side, 1);
        assert_eq!(depth.price, 185.50);
        assert_eq!(depth.size, 100.0);
        assert!(depth.smart_depth);
    }

    #[test]
    fn test_decode_market_depth_l2_proto_default_smart_depth() {
        let proto_msg = crate::proto::MarketDepthL2 {
            req_id: Some(9000),
            market_depth_data: Some(crate::proto::MarketDepthData {
                position: Some(0),
                operation: Some(1),
                side: Some(1),
                price: Some(185.50),
                size: Some("100".into()),
                market_maker: Some("ISLAND".into()),
                is_smart_depth: None,
            }),
        };
        let depth = decode_market_depth_l2_proto(&proto_msg.encode_to_vec()).expect("decode failed");
        assert!(!depth.smart_depth, "missing smart_depth flag should default to false");
    }

    #[test]
    fn test_decode_market_depth_exchanges_proto() {
        let proto_msg = crate::proto::MarketDepthExchanges {
            depth_market_data_descriptions: vec![
                crate::proto::DepthMarketDataDescription {
                    exchange: Some("ISLAND".into()),
                    sec_type: Some("STK".into()),
                    listing_exch: Some("NASDAQ".into()),
                    service_data_type: Some("DEEP2".into()),
                    agg_group: Some(1),
                },
                crate::proto::DepthMarketDataDescription {
                    exchange: Some("NYSE".into()),
                    sec_type: Some("STK".into()),
                    listing_exch: Some("NYSE".into()),
                    service_data_type: Some("DEEP".into()),
                    agg_group: Some(1),
                },
            ],
        };

        let exchanges = decode_market_depth_exchanges_proto(&proto_msg.encode_to_vec()).expect("decode failed");
        assert_eq!(exchanges.len(), 2);
        assert_eq!(exchanges[0].exchange_name, "ISLAND");
        assert_eq!(exchanges[0].listing_exchange, "NASDAQ");
        assert_eq!(exchanges[0].service_data_type, "DEEP2");
        assert_eq!(exchanges[0].aggregated_group, Some("1".to_string()));
        assert_eq!(exchanges[1].exchange_name, "NYSE");
    }

    #[test]
    fn test_decode_market_depth_exchanges_text_path() {
        // Stays dual-format until floor 213; text path still active.
        let mut message = ResponseMessage::from("71\02\0ISLAND\0STK\0NASDAQ\0DEEP2\01\0NYSE\0STK\0NYSE\0DEEP\01\0");
        let exchanges = decode_market_depth_exchanges(server_versions::SERVICE_DATA_TYPE, &mut message).expect("decode failed");
        assert_eq!(exchanges.len(), 2);
        assert_eq!(exchanges[0].exchange_name, "ISLAND");
        assert_eq!(exchanges[0].service_data_type, "DEEP2");
    }

    #[test]
    fn test_decode_market_depth_exchanges_text_path_old_version() {
        let mut message = ResponseMessage::from("71\02\0ISLAND\0STK\01\0NYSE\0STK\00\0");
        let exchanges = decode_market_depth_exchanges(server_versions::SERVICE_DATA_TYPE - 1, &mut message).expect("decode failed");
        assert_eq!(exchanges.len(), 2);
        let first = &exchanges[0];
        assert_eq!(first.exchange_name, "ISLAND");
        assert_eq!(first.service_data_type, "Deep2");
        assert_eq!(first.listing_exchange, "");
        assert_eq!(first.aggregated_group, None);
    }
}

mod tick_price_tests {
    use super::*;

    fn fixture(tick_type: i32, attr_mask: i32) -> TickPriceResponse {
        tick_price().tick_type(tick_type).price(150.25).size(100.0).attr_mask(attr_mask)
    }

    #[test]
    fn test_decode_tick_price_proto_with_size() {
        // TickType::Bid = 1, should produce PriceSize with BidSize.
        // attr_mask 0x5 = can_auto_execute + pre_open
        let result = decode_tick_price_proto(&fixture(1, 0x5).encode_proto()).expect("decode failed");
        match result {
            TickTypes::PriceSize(ps) => {
                assert_eq!(ps.price_tick_type, TickType::Bid);
                assert_eq!(ps.price, 150.25);
                assert_eq!(ps.size, 100.0);
                assert_eq!(ps.size_tick_type, TickType::BidSize);
                assert!(ps.attributes.can_auto_execute);
                assert!(!ps.attributes.past_limit);
                assert!(ps.attributes.pre_open);
            }
            _ => panic!("expected PriceSize variant"),
        }
    }

    #[test]
    fn test_decode_tick_price_proto_unknown_type() {
        // TickType 99 → Unknown size tick type → returns Price variant.
        let proto_msg = tick_price().tick_type(99).price(42.0).size(10.0).attr_mask(0x2).encode_proto();
        match decode_tick_price_proto(&proto_msg).expect("decode failed") {
            TickTypes::Price(tp) => {
                assert_eq!(tp.price, 42.0);
                assert!(tp.attributes.past_limit);
            }
            _ => panic!("expected Price variant for unknown tick type"),
        }
    }

    #[test]
    fn test_decode_tick_price_proto_no_size() {
        // size missing → Price variant (no size companion tick).
        let bytes = tick_price().tick_type(1).price(150.25).encode_proto();
        match decode_tick_price_proto(&bytes).expect("decode failed") {
            TickTypes::Price(tp) => assert_eq!(tp.tick_type, TickType::Bid),
            _ => panic!("expected Price variant when size missing"),
        }
    }
}

mod tick_size_tests {
    use super::*;

    fn fixture(tick_type: i32) -> TickSizeResponse {
        tick_size().tick_type(tick_type).size(500.0)
    }

    #[test]
    fn test_decode_tick_size_proto() {
        let result = decode_tick_size_proto(&fixture(0).encode_proto()).expect("decode failed");
        assert_eq!(result.tick_type, TickType::BidSize);
        assert_eq!(result.size, 500.0);
    }

    #[test]
    fn test_decode_tick_size_proto_all_types() {
        for (type_id, expected) in [
            (0, TickType::BidSize),
            (3, TickType::AskSize),
            (5, TickType::LastSize),
            (8, TickType::Volume),
        ] {
            let bytes = tick_size().tick_type(type_id).size(100.0).encode_proto();
            let tick = decode_tick_size_proto(&bytes).expect("decode failed");
            assert_eq!(tick.tick_type, expected, "type_id {type_id}");
        }
    }
}

mod tick_string_tests {
    use super::*;

    fn fixture() -> TickStringResponse {
        tick_string().tick_type(45).value("1681133400")
    }

    #[test]
    fn test_decode_tick_string_proto() {
        let result = decode_tick_string_proto(&fixture().encode_proto()).expect("decode failed");
        assert_eq!(result.tick_type, TickType::LastTimestamp);
        assert_eq!(result.value, "1681133400");
    }
}

mod tick_generic_tests {
    use super::*;

    fn fixture() -> TickGenericResponse {
        tick_generic().tick_type(49).value(0.0)
    }

    #[test]
    fn test_decode_tick_generic_proto() {
        let result = decode_tick_generic_proto(&fixture().encode_proto()).expect("decode failed");
        assert_eq!(result.tick_type, TickType::Halted);
        assert_eq!(result.value, 0.0);
    }
}

mod tick_efp_tests {
    use super::*;

    #[test]
    fn test_decode_tick_efp() {
        let mut message = ResponseMessage::from("4\0\09000\038\02.5\0+2.50\0100.0\030\020230315\00.5\00.75\0");
        let tick = decode_tick_efp(&mut message).expect("decode failed");
        assert_eq!(tick.tick_type, TickType::BidEfpComputation);
        assert_eq!(tick.basis_points, 2.5);
        assert_eq!(tick.formatted_basis_points, "+2.50");
        assert_eq!(tick.implied_futures_price, 100.0);
        assert_eq!(tick.hold_days, 30);
        assert_eq!(tick.future_last_trade_date, "20230315");
        assert_eq!(tick.dividend_impact, 0.5);
        assert_eq!(tick.dividends_to_last_trade_date, 0.75);
    }

    #[test]
    fn test_decode_tick_efp_types() {
        // TickEFP has no proto encoding on the server side; stays text-only.
        for (type_id, expected) in [
            (38, TickType::BidEfpComputation),
            (39, TickType::AskEfpComputation),
            (40, TickType::LastEfpComputation),
            (41, TickType::OpenEfpComputation),
            (42, TickType::HighEfpComputation),
            (43, TickType::LowEfpComputation),
            (44, TickType::CloseEfpComputation),
        ] {
            let mut message = ResponseMessage::from(format!("4\0\09000\0{type_id}\02.5\0+2.50\0100.0\030\020230315\00.5\00.75\0").as_str());
            let tick = decode_tick_efp(&mut message).expect("decode failed");
            assert_eq!(tick.tick_type, expected, "type_id {type_id}");
        }
    }
}

mod tick_option_computation_tests {
    use super::*;

    #[test]
    fn test_decode_tick_option_computation_proto() {
        let proto_msg = crate::proto::TickOptionComputation {
            req_id: Some(1),
            tick_type: Some(13), // ModelOption
            tick_attrib: Some(1),
            implied_vol: Some(0.25),
            delta: Some(0.5),
            opt_price: Some(5.0),
            pv_dividend: Some(0.1),
            gamma: Some(0.03),
            vega: Some(0.15),
            theta: Some(-0.05),
            und_price: Some(150.0),
        };

        let result = decode_tick_option_computation_proto(&proto_msg.encode_to_vec()).expect("decode failed");
        assert_eq!(result.field, TickType::ModelOption);
        assert_eq!(result.tick_attribute, Some(1));
        assert_eq!(result.implied_volatility, Some(0.25));
        assert_eq!(result.delta, Some(0.5));
        assert_eq!(result.option_price, Some(5.0));
        assert_eq!(result.present_value_dividend, Some(0.1));
        assert_eq!(result.gamma, Some(0.03));
        assert_eq!(result.vega, Some(0.15));
        assert_eq!(result.theta, Some(-0.05));
        assert_eq!(result.underlying_price, Some(150.0));
    }

    #[test]
    fn test_decode_tick_option_computation_proto_max_to_none() {
        // f64::MAX collapses to None via optional_f64.
        let proto_msg = crate::proto::TickOptionComputation {
            req_id: Some(1),
            tick_type: Some(13),
            implied_vol: Some(f64::MAX),
            ..Default::default()
        };
        let result = decode_tick_option_computation_proto(&proto_msg.encode_to_vec()).expect("decode failed");
        assert_eq!(result.implied_volatility, None);
    }
}

mod tick_request_parameters_tests {
    use super::*;

    #[test]
    fn test_decode_tick_request_parameters_proto() {
        let proto_msg = crate::proto::TickReqParams {
            req_id: Some(9000),
            min_tick: Some("0.01".into()),
            bbo_exchange: Some("ISLAND".into()),
            snapshot_permissions: Some(2),
            ..Default::default()
        };

        let result = decode_tick_request_parameters_proto(&proto_msg.encode_to_vec()).expect("decode failed");
        assert_eq!(result.min_tick, 0.01);
        assert_eq!(result.bbo_exchange, "ISLAND");
        assert_eq!(result.snapshot_permissions, 2);
    }
}

mod market_data_type_tests {
    use super::*;
    use crate::subscriptions::common::StreamDecoder;

    #[test]
    fn test_decode_market_data_type_proto_helper() {
        let proto_msg = crate::proto::MarketDataType {
            req_id: Some(9000),
            market_data_type: Some(3),
        };
        assert_eq!(
            decode_market_data_type_proto(&proto_msg.encode_to_vec()).unwrap(),
            MarketDataType::Delayed
        );

        // Forward-compat: out-of-range int → Unknown (no error).
        let proto_msg = crate::proto::MarketDataType {
            req_id: Some(9000),
            market_data_type: Some(99),
        };
        assert_eq!(
            decode_market_data_type_proto(&proto_msg.encode_to_vec()).unwrap(),
            MarketDataType::Unknown
        );
    }

    #[test]
    fn test_decode_market_data_type_proto_through_tick_types() {
        // Live TWS sends MarketDataType as protobuf (msg_id 258 → real_type 58).
        let proto_msg = crate::proto::MarketDataType {
            req_id: Some(9000),
            market_data_type: Some(3),
        };
        let mut message = ResponseMessage::from_protobuf(
            crate::messages::IncomingMessages::MarketDataType as i32,
            proto_msg.encode_to_vec(),
            server_versions::PROTOBUF,
        );
        let context = DecoderContext::new(server_versions::PROTOBUF, None);

        match TickTypes::decode(&context, &mut message).expect("proto decode failed") {
            TickTypes::MarketDataType(MarketDataType::Delayed) => {}
            other => panic!("expected MarketDataType(Delayed), got {other:?}"),
        }
    }

    #[test]
    fn test_decode_tick_req_params_proto_through_tick_types() {
        let proto_msg = crate::proto::TickReqParams {
            req_id: Some(9000),
            min_tick: Some("0.01".into()),
            bbo_exchange: Some("ISLAND".into()),
            snapshot_permissions: Some(2),
            ..Default::default()
        };
        let mut message = ResponseMessage::from_protobuf(
            crate::messages::IncomingMessages::TickReqParams as i32,
            proto_msg.encode_to_vec(),
            server_versions::PROTOBUF,
        );
        let context = DecoderContext::new(server_versions::PROTOBUF, None);

        match TickTypes::decode(&context, &mut message).expect("proto decode failed") {
            TickTypes::RequestParameters(p) => {
                assert_eq!(p.min_tick, 0.01);
                assert_eq!(p.bbo_exchange, "ISLAND");
                assert_eq!(p.snapshot_permissions, 2);
            }
            other => panic!("expected RequestParameters, got {other:?}"),
        }
    }

    #[test]
    fn test_tick_types_decode_unknown_message_type_skips() {
        // Unknown message types must skip-classify (UnexpectedResponse), not terminate.
        let mut message = ResponseMessage::from("92\0\0AccountCode\0DU12345\0\0DU12345\0");
        let context = DecoderContext::new(0, None);

        match TickTypes::decode(&context, &mut message) {
            Err(Error::UnexpectedResponse(_)) => {}
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }
}
