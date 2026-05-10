use super::*;
use crate::messages::ResponseMessage;
use crate::subscriptions::DecoderContext;
use time::OffsetDateTime;

fn encode<M: prost::Message>(msg: &M) -> Vec<u8> {
    let mut bytes = Vec::new();
    msg.encode(&mut bytes).unwrap();
    bytes
}

#[cfg(test)]
mod realtime_bar_tests {
    use super::*;

    fn proto_bar() -> crate::proto::RealTimeBarTick {
        crate::proto::RealTimeBarTick {
            req_id: Some(9000),
            time: Some(1678323335),
            open: Some(4028.75),
            high: Some(4029.00),
            low: Some(4028.25),
            close: Some(4028.50),
            volume: Some("2".into()),
            wap: Some("4026.75".into()),
            count: Some(1),
        }
    }

    #[test]
    fn test_decode_realtime_bar_proto() {
        let bar = decode_realtime_bar_proto(&encode(&proto_bar())).expect("decode failed");

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
            encode(&proto_bar()),
            server_versions::PROTOBUF_HISTORICAL_DATA,
        );
        let bar = decode_realtime_bar(&mut message).expect("decode failed");
        assert_eq!(bar.open, 4028.75);
    }

    #[test]
    fn test_decode_realtime_bar_invalid_proto_bytes() {
        // Garbage bytes — prost decode error.
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

#[cfg(test)]
mod trade_tick_tests {
    use super::*;

    fn proto_trade(tick_type: i32) -> crate::proto::TickByTickData {
        crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(tick_type),
            tick: Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickLast(
                crate::proto::HistoricalTickLast {
                    time: Some(1678740829),
                    tick_attrib_last: Some(crate::proto::TickAttribLast {
                        past_limit: Some(false),
                        unreported: Some(true),
                    }),
                    price: Some(3895.25),
                    size: Some("7".into()),
                    exchange: Some("NASDAQ".into()),
                    special_conditions: Some("Regular".into()),
                },
            )),
        }
    }

    #[test]
    fn test_decode_trade_tick_proto_last() {
        let trade = decode_trade_tick_proto(&encode(&proto_trade(1))).expect("decode failed");
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
        // tick_type 2 = AllLast.
        let trade = decode_trade_tick_proto(&encode(&proto_trade(2))).expect("decode failed");
        assert_eq!(trade.tick_type, "2");
    }

    #[test]
    fn test_decode_trade_tick_proto_invalid_type() {
        // tick_type 3 = BidAsk — wrong feed for the trade decoder.
        let result = decode_trade_tick_proto(&encode(&proto_trade(3)));
        let err = result.expect_err("should reject bid/ask tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }

    #[test]
    fn test_decode_trade_tick_proto_missing_payload() {
        let msg = crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(1),
            tick: None,
        };
        let result = decode_trade_tick_proto(&encode(&msg));
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_trade_tick_through_wrapper() {
        let mut message = ResponseMessage::from_protobuf(
            crate::messages::IncomingMessages::TickByTick as i32,
            encode(&proto_trade(1)),
            server_versions::PROTOBUF_HISTORICAL_DATA,
        );
        let trade = decode_trade_tick(&mut message).expect("decode failed");
        assert_eq!(trade.price, 3895.25);
    }
}

#[cfg(test)]
mod bid_ask_tests {
    use super::*;

    fn proto_bid_ask(mask: u32) -> crate::proto::TickByTickData {
        crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(3),
            tick: Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickBidAsk(
                crate::proto::HistoricalTickBidAsk {
                    time: Some(1678745793),
                    tick_attrib_bid_ask: Some(crate::proto::TickAttribBidAsk {
                        bid_past_low: Some(mask & 0x1 != 0),
                        ask_past_high: Some(mask & 0x2 != 0),
                    }),
                    price_bid: Some(3895.50),
                    price_ask: Some(3896.00),
                    size_bid: Some("9".into()),
                    size_ask: Some("11".into()),
                },
            )),
        }
    }

    #[test]
    fn test_decode_bid_ask_proto_basic() {
        let bid_ask = decode_bid_ask_tick_proto(&encode(&proto_bid_ask(3))).expect("decode failed");
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
            let bid_ask = decode_bid_ask_tick_proto(&encode(&proto_bid_ask(mask))).expect("decode failed");
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
        let err = decode_bid_ask_tick_proto(&encode(&msg)).expect_err("should reject last tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }
}

#[cfg(test)]
mod mid_point_tests {
    use super::*;

    fn proto_mid_point() -> crate::proto::TickByTickData {
        crate::proto::TickByTickData {
            req_id: Some(9000),
            tick_type: Some(4),
            tick: Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickMidPoint(
                crate::proto::HistoricalTick {
                    time: Some(1678740829),
                    price: Some(3895.375),
                    size: Some("0".into()),
                },
            )),
        }
    }

    #[test]
    fn test_decode_mid_point_tick_proto() {
        let mid = decode_mid_point_tick_proto(&encode(&proto_mid_point())).expect("decode failed");
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
        let err = decode_mid_point_tick_proto(&encode(&msg)).expect_err("should reject last tick type");
        assert!(err.to_string().contains("Unexpected tick_type"));
    }
}

#[cfg(test)]
mod market_depth_tests {
    use super::*;

    fn proto_depth(side: i32, operation: i32) -> crate::proto::MarketDepth {
        crate::proto::MarketDepth {
            req_id: Some(9000),
            market_depth_data: Some(crate::proto::MarketDepthData {
                position: Some(0),
                operation: Some(operation),
                side: Some(side),
                price: Some(185.50),
                size: Some("100".into()),
                market_maker: None,
                is_smart_depth: None,
            }),
        }
    }

    #[test]
    fn test_decode_market_depth_proto_basic() {
        let depth = decode_market_depth_proto(&encode(&proto_depth(1, 1))).expect("decode failed");
        assert_eq!(depth.position, 0);
        assert_eq!(depth.operation, 1);
        assert_eq!(depth.side, 1);
        assert_eq!(depth.price, 185.50);
        assert_eq!(depth.size, 100.0);
    }

    #[test]
    fn test_decode_market_depth_proto_operations() {
        for op in [0, 1, 2] {
            let depth = decode_market_depth_proto(&encode(&proto_depth(1, op))).expect("decode failed");
            assert_eq!(depth.operation, op);
        }
    }

    #[test]
    fn test_decode_market_depth_proto_sides() {
        for side in [0, 1] {
            let depth = decode_market_depth_proto(&encode(&proto_depth(side, 0))).expect("decode failed");
            assert_eq!(depth.side, side);
        }
    }

    #[test]
    fn test_decode_market_depth_proto_missing_data() {
        let msg = crate::proto::MarketDepth {
            req_id: Some(9000),
            market_depth_data: None,
        };
        let err = decode_market_depth_proto(&encode(&msg)).expect_err("missing data should error");
        assert!(err.to_string().contains("missing market_depth_data"));
    }

    #[test]
    fn test_decode_market_depth_l2_proto() {
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
        let depth = decode_market_depth_l2_proto(&encode(&proto_msg)).expect("decode failed");

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
        let depth = decode_market_depth_l2_proto(&encode(&proto_msg)).expect("decode failed");
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

        let exchanges = decode_market_depth_exchanges_proto(&encode(&proto_msg)).expect("decode failed");
        assert_eq!(exchanges.len(), 2);
        assert_eq!(exchanges[0].exchange_name, "ISLAND");
        assert_eq!(exchanges[0].listing_exchange, "NASDAQ");
        assert_eq!(exchanges[0].service_data_type, "DEEP2");
        assert_eq!(exchanges[0].aggregated_group, Some("1".to_string()));
        assert_eq!(exchanges[1].exchange_name, "NYSE");
    }

    #[test]
    fn test_decode_market_depth_exchanges_text_path() {
        // MktDepthExchanges stays dual-format until floor 213; text path still active at 210.
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

#[cfg(test)]
mod tick_price_tests {
    use super::*;

    #[test]
    fn test_decode_tick_price_proto_with_size() {
        // TickType::Bid = 1, should produce PriceSize with BidSize.
        let proto_msg = crate::proto::TickPrice {
            req_id: Some(1),
            tick_type: Some(1),
            price: Some(150.25),
            size: Some("100".into()),
            attr_mask: Some(0x5), // can_auto_execute + pre_open
        };

        let result = decode_tick_price_proto(&encode(&proto_msg)).expect("decode failed");
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
        // TickType 99 => Unknown size tick type => returns Price variant.
        let proto_msg = crate::proto::TickPrice {
            req_id: Some(1),
            tick_type: Some(99),
            price: Some(42.0),
            size: Some("10".into()),
            attr_mask: Some(0x2),
        };

        let result = decode_tick_price_proto(&encode(&proto_msg)).expect("decode failed");
        match result {
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
        let proto_msg = crate::proto::TickPrice {
            req_id: Some(1),
            tick_type: Some(1),
            price: Some(150.25),
            size: None,
            attr_mask: Some(0),
        };

        match decode_tick_price_proto(&encode(&proto_msg)).expect("decode failed") {
            TickTypes::Price(tp) => assert_eq!(tp.tick_type, TickType::Bid),
            _ => panic!("expected Price variant when size missing"),
        }
    }
}

#[cfg(test)]
mod tick_size_tests {
    use super::*;

    #[test]
    fn test_decode_tick_size_proto() {
        let proto_msg = crate::proto::TickSize {
            req_id: Some(1),
            tick_type: Some(0), // BidSize
            size: Some("500".into()),
        };
        let result = decode_tick_size_proto(&encode(&proto_msg)).expect("decode failed");
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
            let proto_msg = crate::proto::TickSize {
                req_id: Some(1),
                tick_type: Some(type_id),
                size: Some("100".into()),
            };
            let tick = decode_tick_size_proto(&encode(&proto_msg)).expect("decode failed");
            assert_eq!(tick.tick_type, expected, "type_id {type_id}");
        }
    }
}

#[cfg(test)]
mod tick_string_tests {
    use super::*;

    #[test]
    fn test_decode_tick_string_proto() {
        let proto_msg = crate::proto::TickString {
            req_id: Some(1),
            tick_type: Some(45), // LastTimestamp
            value: Some("1681133400".into()),
        };
        let result = decode_tick_string_proto(&encode(&proto_msg)).expect("decode failed");
        assert_eq!(result.tick_type, TickType::LastTimestamp);
        assert_eq!(result.value, "1681133400");
    }
}

#[cfg(test)]
mod tick_generic_tests {
    use super::*;

    #[test]
    fn test_decode_tick_generic_proto() {
        let proto_msg = crate::proto::TickGeneric {
            req_id: Some(1),
            tick_type: Some(49), // Halted
            value: Some(0.0),
        };
        let result = decode_tick_generic_proto(&encode(&proto_msg)).expect("decode failed");
        assert_eq!(result.tick_type, TickType::Halted);
        assert_eq!(result.value, 0.0);
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

        let result = decode_tick_option_computation_proto(&encode(&proto_msg)).expect("decode failed");
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
        let result = decode_tick_option_computation_proto(&encode(&proto_msg)).expect("decode failed");
        assert_eq!(result.implied_volatility, None);
    }
}

#[cfg(test)]
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

        let result = decode_tick_request_parameters_proto(&encode(&proto_msg)).expect("decode failed");
        assert_eq!(result.min_tick, 0.01);
        assert_eq!(result.bbo_exchange, "ISLAND");
        assert_eq!(result.snapshot_permissions, 2);
    }
}

#[cfg(test)]
mod market_data_type_tests {
    use super::*;
    use crate::subscriptions::common::StreamDecoder;

    #[test]
    fn test_decode_market_data_type_proto_helper() {
        let proto_msg = crate::proto::MarketDataType {
            req_id: Some(9000),
            market_data_type: Some(3),
        };
        assert_eq!(decode_market_data_type_proto(&encode(&proto_msg)).unwrap(), MarketDataType::Delayed);

        // Forward-compat: out-of-range int → Unknown (no error).
        let proto_msg = crate::proto::MarketDataType {
            req_id: Some(9000),
            market_data_type: Some(99),
        };
        assert_eq!(decode_market_data_type_proto(&encode(&proto_msg)).unwrap(), MarketDataType::Unknown);
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
            encode(&proto_msg),
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
            encode(&proto_msg),
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
