use super::*;
use crate::messages::ResponseMessage;
use time::OffsetDateTime;

#[cfg(test)]
mod realtime_bar_tests {
    use super::*;

    #[test]
    fn test_decode_realtime_bar() {
        let mut message = ResponseMessage::from("50\0\09000\01678323335\04028.75\04029.00\04028.25\04028.50\02\04026.75\01\0");

        let bar = decode_realtime_bar(&mut message).expect("Failed to decode realtime bar");

        assert_eq!(bar.date, OffsetDateTime::from_unix_timestamp(1678323335).unwrap(), "Wrong timestamp");
        assert_eq!(bar.open, 4028.75, "Wrong open price");
        assert_eq!(bar.high, 4029.00, "Wrong high price");
        assert_eq!(bar.low, 4028.25, "Wrong low price");
        assert_eq!(bar.close, 4028.50, "Wrong close price");
        assert_eq!(bar.volume, 2.0, "Wrong volume");
        assert_eq!(bar.wap, 4026.75, "Wrong WAP");
        assert_eq!(bar.count, 1, "Wrong count");
    }

    #[test]
    fn test_decode_realtime_bar_invalid_format() {
        let mut message = ResponseMessage::from("50\0\09000\0invalid_timestamp\04028.75\04029.00\04028.25\04028.50\02\04026.75\01\0");

        let result = decode_realtime_bar(&mut message);
        assert!(result.is_err(), "Should fail with invalid timestamp");
    }

    #[test]
    fn test_decode_realtime_bar_empty_message() {
        let mut message = ResponseMessage::from("");
        let result = decode_realtime_bar(&mut message);
        assert!(result.is_err(), "Should fail with empty message");
    }
}

#[cfg(test)]
mod trade_tick_tests {
    use super::*;

    #[test]
    fn test_decode_trade_tick() {
        let mut message = ResponseMessage::from("99\09000\01\01678740829\03895.25\07\02\0NASDAQ\0Regular\0");

        let trade = decode_trade_tick(&mut message).expect("Failed to decode trade tick");

        assert_eq!(trade.tick_type, "1", "Wrong tick type");
        assert_eq!(trade.time, OffsetDateTime::from_unix_timestamp(1678740829).unwrap(), "Wrong timestamp");
        assert_eq!(trade.price, 3895.25, "Wrong price");
        assert_eq!(trade.size, 7.0, "Wrong size");
        assert_eq!(trade.trade_attribute.past_limit, false, "Wrong past limit flag");
        assert_eq!(trade.trade_attribute.unreported, true, "Wrong unreported flag");
        assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange");
        assert_eq!(trade.special_conditions, "Regular", "Wrong special conditions");
    }

    #[test]
    fn test_decode_trade_tick_invalid_type() {
        let mut message = ResponseMessage::from("99\09000\03\01678740829\03895.25\07\02\0NASDAQ\0Regular\0");

        let result = decode_trade_tick(&mut message);
        assert!(result.is_err(), "Should fail with invalid tick type");
        assert!(result.unwrap_err().to_string().contains("Unexpected tick_type"));
    }

    #[test]
    fn test_decode_trade_tick_with_empty_fields() {
        let mut message = ResponseMessage::from("99\09000\01\01678740829\03895.25\07\02\0\0\0");

        let trade = decode_trade_tick(&mut message).expect("Failed to decode trade tick");

        assert_eq!(trade.exchange, "", "Exchange should be empty");
        assert_eq!(trade.special_conditions, "", "Special conditions should be empty");
    }
}

#[cfg(test)]
mod bid_ask_tests {
    use super::*;

    #[test]
    fn test_decode_bid_ask_basic() {
        let mut message = ResponseMessage::from("99\09000\03\01678745793\03895.50\03896.00\09\011\03\0");

        let bid_ask = decode_bid_ask_tick(&mut message).expect("Failed to decode bid/ask tick");

        assert_eq!(bid_ask.time, OffsetDateTime::from_unix_timestamp(1678745793).unwrap(), "Wrong timestamp");
        assert_eq!(bid_ask.bid_price, 3895.50, "Wrong bid price");
        assert_eq!(bid_ask.ask_price, 3896.00, "Wrong ask price");
        assert_eq!(bid_ask.bid_size, 9, "Wrong bid size");
        assert_eq!(bid_ask.ask_size, 11, "Wrong ask size");
        assert_eq!(bid_ask.bid_ask_attribute.bid_past_low, true, "Wrong bid past low flag");
        assert_eq!(bid_ask.bid_ask_attribute.ask_past_high, true, "Wrong ask past high flag");
    }

    #[test]
    fn test_decode_bid_ask_attributes() {
        // Test different attribute mask combinations
        let test_cases = vec![
            (0, false, false), // No flags
            (1, true, false),  // Bid past low only
            (2, false, true),  // Ask past high only
            (3, true, true),   // Both flags
        ];

        for (mask, expected_bid_past_low, expected_ask_past_high) in test_cases {
            let mut message = ResponseMessage::from(format!("99\09000\03\01678745793\03895.50\03896.00\09\011\0{}\0", mask).as_str());

            let bid_ask = decode_bid_ask_tick(&mut message).expect("Failed to decode bid/ask tick");

            assert_eq!(
                bid_ask.bid_ask_attribute.bid_past_low, expected_bid_past_low,
                "Wrong bid past low flag for mask {}",
                mask
            );
            assert_eq!(
                bid_ask.bid_ask_attribute.ask_past_high, expected_ask_past_high,
                "Wrong ask past high flag for mask {}",
                mask
            );
        }
    }

    #[test]
    fn test_decode_bid_ask_invalid_type() {
        let mut message = ResponseMessage::from("99\09000\01\01678745793\03895.50\03896.00\09\011\03\0");

        let result = decode_bid_ask_tick(&mut message);
        assert!(result.is_err(), "Should fail with invalid tick type");
        assert!(result.unwrap_err().to_string().contains("Unexpected tick_type"));
    }
}

#[cfg(test)]
mod market_depth_tests {
    use super::*;

    #[test]
    fn test_decode_market_depth_basic() {
        let mut message = ResponseMessage::from("12\0\09000\00\01\01\0185.50\0100\0");

        let depth = decode_market_depth(&mut message).expect("Failed to decode market depth");

        assert_eq!(depth.position, 0, "Wrong position");
        assert_eq!(depth.operation, 1, "Wrong operation");
        assert_eq!(depth.side, 1, "Wrong side");
        assert_eq!(depth.price, 185.50, "Wrong price");
        assert_eq!(depth.size, 100.0, "Wrong size");
    }

    #[test]
    fn test_decode_market_depth_operations() {
        // Test all valid operation types
        let operations = vec![0, 1, 2]; // Insert, Update, Delete

        for op in operations {
            let mut message = ResponseMessage::from(format!("12\0\09000\00\0{}\01\0185.50\0100\0", op).as_str());

            let depth = decode_market_depth(&mut message).expect("Failed to decode market depth");
            assert_eq!(depth.operation, op, "Wrong operation value for op {}", op);
        }
    }

    #[test]
    fn test_decode_market_depth_sides() {
        // Test both valid sides (ask=0, bid=1)
        let sides = vec![0, 1];

        for side in sides {
            let mut message = ResponseMessage::from(format!("12\0\09000\00\01\0{}\0185.50\0100\0", side).as_str());

            let depth = decode_market_depth(&mut message).expect("Failed to decode market depth");
            assert_eq!(depth.side, side, "Wrong side value for side {}", side);
        }
    }

    #[test]
    fn test_decode_market_depth_l2() {
        let mut message = ResponseMessage::from("13\0\09000\00\0ISLAND\01\01\0185.50\0100\01\0");

        let depth = decode_market_depth_l2(server_versions::SMART_DEPTH, &mut message).expect("Failed to decode market depth L2");

        assert_eq!(depth.position, 0, "Wrong position");
        assert_eq!(depth.market_maker, "ISLAND", "Wrong market maker");
        assert_eq!(depth.operation, 1, "Wrong operation");
        assert_eq!(depth.side, 1, "Wrong side");
        assert_eq!(depth.price, 185.50, "Wrong price");
        assert_eq!(depth.size, 100.0, "Wrong size");
        assert_eq!(depth.smart_depth, true, "Wrong smart depth flag");
    }

    #[test]
    fn test_decode_market_depth_l2_version_handling() {
        // Test pre-SMART_DEPTH version
        let mut message = ResponseMessage::from("13\0\09000\00\0ISLAND\01\01\0185.50\0100\0");

        let depth = decode_market_depth_l2(server_versions::SMART_DEPTH - 1, &mut message).expect("Failed to decode market depth L2");
        assert_eq!(depth.smart_depth, false, "Should default to false for old server version");

        // Test with SMART_DEPTH version
        let mut message = ResponseMessage::from("13\0\09000\00\0ISLAND\01\01\0185.50\0100\01\0");
        let depth = decode_market_depth_l2(server_versions::SMART_DEPTH, &mut message).expect("Failed to decode market depth L2");
        assert_eq!(depth.smart_depth, true, "Should read smart_depth flag for new server version");
    }

    #[test]
    fn test_decode_market_depth_exchanges() {
        let mut message = ResponseMessage::from("71\02\0ISLAND\0STK\0NASDAQ\0DEEP2\01\0NYSE\0STK\0NYSE\0DEEP\01\0");

        let exchanges =
            decode_market_depth_exchanges(server_versions::SERVICE_DATA_TYPE, &mut message).expect("Failed to decode market depth exchanges");

        assert_eq!(exchanges.len(), 2, "Wrong number of exchanges");

        // Check first exchange
        let first = &exchanges[0];
        assert_eq!(first.exchange_name, "ISLAND", "Wrong exchange name");
        assert_eq!(first.security_type, "STK", "Wrong security type");
        assert_eq!(first.listing_exchange, "NASDAQ", "Wrong listing exchange");
        assert_eq!(first.service_data_type, "DEEP2", "Wrong service data type");
        assert_eq!(first.aggregated_group, Some("1".to_string()), "Wrong aggregated group");

        // Check second exchange
        let second = &exchanges[1];
        assert_eq!(second.exchange_name, "NYSE", "Wrong exchange name");
        assert_eq!(second.security_type, "STK", "Wrong security type");
        assert_eq!(second.listing_exchange, "NYSE", "Wrong listing exchange");
        assert_eq!(second.service_data_type, "DEEP", "Wrong service data type");
        assert_eq!(second.aggregated_group, Some("1".to_string()), "Wrong aggregated group");
    }

    #[test]
    fn test_decode_market_depth_exchanges_old_version() {
        let mut message = ResponseMessage::from("71\02\0ISLAND\0STK\01\0NYSE\0STK\00\0");

        let exchanges =
            decode_market_depth_exchanges(server_versions::SERVICE_DATA_TYPE - 1, &mut message).expect("Failed to decode market depth exchanges");

        assert_eq!(exchanges.len(), 2, "Wrong number of exchanges");

        let first = &exchanges[0];
        assert_eq!(first.exchange_name, "ISLAND", "Wrong exchange name");
        assert_eq!(first.security_type, "STK", "Wrong security type");
        assert_eq!(first.service_data_type, "Deep2", "Wrong service data type");
        assert_eq!(first.listing_exchange, "", "Listing exchange should be empty for old version");
        assert_eq!(first.aggregated_group, None, "Aggregated group should be None for old version");
    }
}

#[cfg(test)]
mod tick_price_tests {
    use super::*;

    #[test]
    fn test_decode_tick_price_basic() {
        let mut message = ResponseMessage::from("1\01\09000\01\0185.50\07\0");

        if let TickTypes::Price(tick) = decode_tick_price(server_versions::PRE_OPEN_BID_ASK, &mut message).expect("Failed to decode tick price") {
            assert_eq!(tick.tick_type, TickType::Bid, "Wrong tick type");
            assert_eq!(tick.price, 185.50, "Wrong price");
            assert_eq!(tick.attributes.can_auto_execute, false, "Wrong can auto execute flag");
            assert_eq!(tick.attributes.past_limit, false, "Wrong past limit flag");
            assert_eq!(tick.attributes.pre_open, false, "Wrong pre open flag");
        } else {
            panic!("Expected TickTypes::Price variant");
        }
    }

    #[test]
    fn test_decode_tick_price_version_handling() {
        let test_cases = vec![
            (server_versions::PAST_LIMIT - 1, false, false, false), // Pre PAST_LIMIT
            (server_versions::PAST_LIMIT, true, true, false),       // Post PAST_LIMIT
            (server_versions::PRE_OPEN_BID_ASK, true, true, true),  // Post PRE_OPEN_BID_ASK
        ];

        for (version, expect_auto_execute, expect_past_limit, expect_pre_open) in test_cases {
            let mut message = ResponseMessage::from("1\02\09000\01\0185.50\0100\07\0");

            if let TickTypes::Price(tick) = decode_tick_price(version, &mut message).expect("Failed to decode tick price") {
                assert_eq!(
                    tick.attributes.can_auto_execute, expect_auto_execute,
                    "Wrong auto execute for version {}",
                    version
                );
                assert_eq!(tick.attributes.past_limit, expect_past_limit, "Wrong past limit for version {}", version);
                assert_eq!(tick.attributes.pre_open, expect_pre_open, "Wrong pre open for version {}", version);
            }
        }
    }

    #[test]
    fn test_decode_tick_price_size() {
        let mut message = ResponseMessage::from("1\02\09000\01\0185.50\0100\07\0");

        if let TickTypes::PriceSize(tick) = decode_tick_price(server_versions::PRE_OPEN_BID_ASK, &mut message).expect("Failed to decode tick price") {
            assert_eq!(tick.price_tick_type, TickType::Bid, "Wrong price tick type");
            assert_eq!(tick.size_tick_type, TickType::BidSize, "Wrong size tick type");
            assert_eq!(tick.price, 185.50, "Wrong price");
            assert_eq!(tick.size, 100.0, "Wrong size");
        } else {
            panic!("Expected TickTypes::PriceSize variant");
        }
    }
}

#[cfg(test)]
mod tick_size_tests {
    use super::*;

    #[test]
    fn test_decode_tick_size() {
        let mut message = ResponseMessage::from("2\0\09000\00\0100\0");

        let tick = decode_tick_size(&mut message).expect("Failed to decode tick size");

        assert_eq!(tick.tick_type, TickType::BidSize, "Wrong tick type");
        assert_eq!(tick.size, 100.0, "Wrong size");
    }

    #[test]
    fn test_decode_tick_size_all_types() {
        let tick_types = vec![
            (0, TickType::BidSize),
            (3, TickType::AskSize),
            (5, TickType::LastSize),
            (8, TickType::Volume),
        ];

        for (type_id, expected_type) in tick_types {
            let mut message = ResponseMessage::from(format!("2\0\09000\0{}\0100\0", type_id).as_str());

            let tick = decode_tick_size(&mut message).expect("Failed to decode tick size");
            assert_eq!(tick.tick_type, expected_type, "Wrong tick type for type_id {}", type_id);
        }
    }
}

#[cfg(test)]
mod tick_string_tests {
    use super::*;

    #[test]
    fn test_decode_tick_string() {
        let mut message = ResponseMessage::from("3\0\09000\045\02023-03-13 09:30:00\0");

        let tick = decode_tick_string(&mut message).expect("Failed to decode tick string");

        assert_eq!(tick.tick_type, TickType::LastTimestamp, "Wrong tick type");
        assert_eq!(tick.value, "2023-03-13 09:30:00", "Wrong value");
    }

    #[test]
    fn test_decode_tick_string_types() {
        let test_cases = vec![
            (0, TickType::BidSize, "ISLAND"),
            (32, TickType::BidExch, "NYSE"),
            (84, TickType::LastExch, "NASDAQ"),
        ];

        for (type_id, expected_type, value) in test_cases {
            let mut message = ResponseMessage::from(format!("3\0\09000\0{}\0{}\0", type_id, value).as_str());

            let tick = decode_tick_string(&mut message).expect("Failed to decode tick string");
            assert_eq!(tick.tick_type, expected_type, "Wrong tick type for type_id {}", type_id);
            assert_eq!(tick.value, value, "Wrong value for type_id {}", type_id);
        }
    }
}

#[cfg(test)]
mod tick_generic_tests {
    use super::*;

    #[test]
    fn test_decode_tick_generic() {
        let mut message = ResponseMessage::from("5\0\09000\023\020.5\0");

        let tick = decode_tick_generic(&mut message).expect("Failed to decode tick generic");

        assert_eq!(tick.tick_type, TickType::OptionHistoricalVol, "Wrong tick type");
        assert_eq!(tick.value, 20.5, "Wrong value");
    }

    #[test]
    fn test_decode_tick_generic_types() {
        let test_cases = vec![
            (23, TickType::OptionHistoricalVol, 20.5),
            (24, TickType::OptionImpliedVol, 15.3),
            (31, TickType::IndexFuturePremium, 1.5),
            (49, TickType::Halted, 1.0),
        ];

        for (type_id, expected_type, value) in test_cases {
            let mut message = ResponseMessage::from(format!("5\0\09000\0{}\0{}\0", type_id, value).as_str());

            let tick = decode_tick_generic(&mut message).expect("Failed to decode tick generic");
            assert_eq!(tick.tick_type, expected_type, "Wrong tick type for type_id {}", type_id);
            assert_eq!(tick.value, value, "Wrong value for type_id {}", type_id);
        }
    }
}

#[cfg(test)]
mod tick_efp_tests {
    use super::*;

    #[test]
    fn test_decode_tick_efp() {
        let mut message = ResponseMessage::from("4\0\09000\038\02.5\0+2.50\0100.0\030\020230315\00.5\00.75\0");

        let tick = decode_tick_efp(&mut message).expect("Failed to decode tick EFP");

        assert_eq!(tick.tick_type, TickType::BidEfpComputation, "Wrong tick type");
        assert_eq!(tick.basis_points, 2.5, "Wrong basis points");
        assert_eq!(tick.formatted_basis_points, "+2.50", "Wrong formatted basis points");
        assert_eq!(tick.implied_futures_price, 100.0, "Wrong implied futures price");
        assert_eq!(tick.hold_days, 30, "Wrong hold days");
        assert_eq!(tick.future_last_trade_date, "20230315", "Wrong future last trade date");
        assert_eq!(tick.dividend_impact, 0.5, "Wrong dividend impact");
        assert_eq!(tick.dividends_to_last_trade_date, 0.75, "Wrong dividends to last trade");
    }

    #[test]
    fn test_decode_tick_efp_types() {
        let test_cases = vec![
            (38, TickType::BidEfpComputation),
            (39, TickType::AskEfpComputation),
            (40, TickType::LastEfpComputation),
            (41, TickType::OpenEfpComputation),
            (42, TickType::HighEfpComputation),
            (43, TickType::LowEfpComputation),
            (44, TickType::CloseEfpComputation),
        ];

        for (type_id, expected_type) in test_cases {
            let mut message = ResponseMessage::from(format!("4\0\09000\0{}\02.5\0+2.50\0100.0\030\020230315\00.5\00.75\0", type_id).as_str());

            let tick = decode_tick_efp(&mut message).expect("Failed to decode tick EFP");
            assert_eq!(tick.tick_type, expected_type, "Wrong tick type for type_id {}", type_id);
        }
    }
}
