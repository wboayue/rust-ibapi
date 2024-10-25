use super::*;
use std::sync::Arc;
use std::sync::RwLock;
use crate::stubs::MessageBusStub;
use crate::messages::OutgoingMessages;
use crate::contracts::contract_samples;
use time::OffsetDateTime;

mod subscription_tests {
    use super::*;

    #[test]
    fn test_realtime_bars() {
        // Setup test message bus with mock responses
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "50|3|9001|1678323335|4028.75|4029.00|4028.25|4028.50|2|4026.75|1|".to_owned(),
                "50|3|9001|1678323340|4028.80|4029.10|4028.30|4028.55|3|4026.80|2|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = contract_samples::future_with_local_symbol();
        let bar_size = BarSize::Sec5;
        let what_to_show = WhatToShow::Trades;
        let use_rth = true;

        // Test subscription creation
        let bars = client.realtime_bars(&contract, bar_size, what_to_show, use_rth);
        assert!(bars.is_ok(), "Failed to create realtime bars subscription");

        // Test receiving data
        let bars = bars.unwrap();
        let received_bars: Vec<Bar> = bars.iter().take(2).collect();

        assert_eq!(received_bars.len(), 2, "Should receive 2 bars");

        // Verify first bar
        assert_eq!(
            received_bars[0].date,
            OffsetDateTime::from_unix_timestamp(1678323335).unwrap(),
            "Wrong timestamp for first bar"
        );
        assert_eq!(received_bars[0].open, 4028.75, "Wrong open price for first bar");
        assert_eq!(received_bars[0].volume, 2.0, "Wrong volume for first bar");

        // Verify second bar
        assert_eq!(
            received_bars[1].date,
            OffsetDateTime::from_unix_timestamp(1678323340).unwrap(),
            "Wrong timestamp for second bar"
        );
        assert_eq!(received_bars[1].open, 4028.80, "Wrong open price for second bar");
        assert_eq!(received_bars[1].volume, 3.0, "Wrong volume for second bar");

        // Verify request messages
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestRealTimeBars.to_field(), "Wrong message type");
        assert_eq!(request[1], "8", "Wrong version");
        assert_eq!(request[16], what_to_show.to_field(), "Wrong what to show value");
        assert_eq!(request[17], use_rth.to_field(), "Wrong use RTH flag");
    }

    #[test]
    fn test_tick_by_tick_all_last() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "99|9001|1|1678740829|3895.25|7|2|NASDAQ|Regular|".to_owned(),
                "99|9001|1|1678740830|3895.50|5|0|NYSE|Regular|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK);
        let contract = contract_samples::simple_future();
        let number_of_ticks = 2;
        let ignore_size = false;

        // Test subscription creation
        let trades = client.tick_by_tick_all_last(&contract, number_of_ticks, ignore_size);
        assert!(trades.is_ok(), "Failed to create tick-by-tick subscription");

        // Test receiving data
        let trades = trades.unwrap();
        let received_trades: Vec<Trade> = trades.iter().take(2).collect();

        assert_eq!(received_trades.len(), 2, "Should receive 2 trades");

        // Verify first trade
        assert_eq!(received_trades[0].price, 3895.25, "Wrong price for first trade");
        assert_eq!(received_trades[0].size, 7, "Wrong size for first trade");
        assert_eq!(received_trades[0].exchange, "NASDAQ", "Wrong exchange for first trade");

        // Verify second trade
        assert_eq!(received_trades[1].price, 3895.50, "Wrong price for second trade");
        assert_eq!(received_trades[1].size, 5, "Wrong size for second trade");
        assert_eq!(received_trades[1].exchange, "NYSE", "Wrong exchange for second trade");

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request[0],
            OutgoingMessages::RequestTickByTickData.to_field(),
            "Wrong message type"
        );
        assert_eq!(request[14], "AllLast", "Wrong tick type");
    }
}

mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_tick_by_tick_request() {
        // Test with old server version
        let client = Client::stubbed(
            Arc::new(MessageBusStub::default()),
            server_versions::TICK_BY_TICK - 1
        );
        let contract = contract_samples::simple_future();
        
        let result = validate_tick_by_tick_request(&client, &contract, 0, false);
        assert!(result.is_err(), "Should fail with old server version");
        
        // Test with new server version but old parameters
        let client = Client::stubbed(
            Arc::new(MessageBusStub::default()),
            server_versions::TICK_BY_TICK
        );
        
        let result = validate_tick_by_tick_request(&client, &contract, 1, true);
        assert!(result.is_err(), "Should fail with new server version but old parameters");

        // Test with new server version and new parameters
        let client = Client::stubbed(
            Arc::new(MessageBusStub::default()),
            server_versions::TICK_BY_TICK_IGNORE_SIZE
        );
        
        let result = validate_tick_by_tick_request(&client, &contract, 1, true);
        assert!(result.is_ok(), "Should succeed with new server version and parameters");
    }

    #[test]
    fn test_what_to_show_display() {
        assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
        assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
        assert_eq!(WhatToShow::Bid.to_string(), "BID");
        assert_eq!(WhatToShow::Ask.to_string(), "ASK");
    }
}

mod market_depth_tests {
    use super::*;

    #[test]
    fn test_market_depth() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "12|9001|0|1|1|185.50|100|".to_owned(),
                "12|9001|1|1|0|185.45|200|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SMART_DEPTH);
        let contract = Contract::stock("AAPL");
        let number_of_rows = 5;
        let is_smart_depth = true;

        // Test subscription creation
        let depth = client.market_depth(&contract, number_of_rows, is_smart_depth);
        assert!(depth.is_ok(), "Failed to create market depth subscription");

        // Test receiving data
        let depth = depth.unwrap();
        let received_depth: Vec<MarketDepths> = depth.iter().take(2).collect();

        assert_eq!(received_depth.len(), 2, "Should receive 2 market depth updates");

        // Verify first update
        if let MarketDepths::MarketDepth(update) = &received_depth[0] {
            assert_eq!(update.position, 0, "Wrong position for first update");
            assert_eq!(update.operation, 1, "Wrong operation for first update");
            assert_eq!(update.side, 1, "Wrong side for first update");
            assert_eq!(update.price, 185.50, "Wrong price for first update");
            assert_eq!(update.size, 100.0, "Wrong size for first update");
        } else {
            panic!("Expected MarketDepth variant");
        }

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request[0],
            OutgoingMessages::RequestMarketDepth.to_field(),
            "Wrong message type"
        );
    }

    #[test]
    fn test_market_depth_exchanges() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "71|2|ISLAND|STK|NASDAQ|DEEP2|1|NYSE|STK|NYSE|DEEP|1|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SERVICE_DATA_TYPE);

        // Test request execution
        let exchanges = market_depth_exchanges(&client);
        assert!(exchanges.is_ok(), "Failed to request market depth exchanges");

        let exchanges = exchanges.unwrap();
        assert_eq!(exchanges.len(), 2, "Should receive 2 exchange descriptions");

        // Verify first exchange
        let first = &exchanges[0];
        assert_eq!(first.exchange_name, "ISLAND", "Wrong exchange name");
        assert_eq!(first.security_type, "STK", "Wrong security type");
        assert_eq!(first.listing_exchange, "NASDAQ", "Wrong listing exchange");
        assert_eq!(first.service_data_type, "DEEP2", "Wrong service data type");
        assert_eq!(first.aggregated_group, Some("1".to_string()), "Wrong aggregated group");

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request[0],
            OutgoingMessages::RequestMktDepthExchanges.to_field(),
            "Wrong message type"
        );
    }
}

mod tick_data_tests {
    use super::*;

    #[test]
    fn test_tick_by_tick_bid_ask() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "99|9001|3|1678745793|3895.50|3896.00|9|11|3|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK);
        let contract = contract_samples::simple_future();
        let number_of_ticks = 1;
        let ignore_size = false;

        // Test subscription creation
        let ticks = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size);
        assert!(ticks.is_ok(), "Failed to create bid/ask subscription");

        // Test receiving data
        let ticks = ticks.unwrap();
        let received_ticks: Vec<BidAsk> = ticks.iter().take(1).collect();

        assert_eq!(received_ticks.len(), 1, "Should receive 1 bid/ask tick");

        // Verify tick data
        let tick = &received_ticks[0];
        assert_eq!(tick.bid_price, 3895.50, "Wrong bid price");
        assert_eq!(tick.ask_price, 3896.00, "Wrong ask price");
        assert_eq!(tick.bid_size, 9, "Wrong bid size");
        assert_eq!(tick.ask_size, 11, "Wrong ask size");

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];
        assert_eq!(request[14], "BidAsk", "Wrong tick type");
    }

    #[test]
    fn test_tick_by_tick_midpoint() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "99|9001|4|1678746113|3896.875|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK);
        let contract = contract_samples::simple_future();
        let number_of_ticks = 1;
        let ignore_size = false;

        // Test subscription creation
        let ticks = client.tick_by_tick_midpoint(&contract, number_of_ticks, ignore_size);
        assert!(ticks.is_ok(), "Failed to create midpoint subscription");

        // Test receiving data
        let ticks = ticks.unwrap();
        let received_ticks: Vec<MidPoint> = ticks.iter().take(1).collect();

        assert_eq!(received_ticks.len(), 1, "Should receive 1 midpoint tick");

        // Verify tick data
        let tick = &received_ticks[0];
        assert_eq!(tick.mid_point, 3896.875, "Wrong midpoint price");

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];
        assert_eq!(request[14], "MidPoint", "Wrong tick type");
    }
}

mod market_data_tests {
    use super::*;
    use crate::contracts::{ComboLeg, DeltaNeutralContract, SecurityType};

    #[test]
    fn test_basic_market_data() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Tick Price message
                "1|2|9001|1|185.50|100|7|".to_owned(),
                // Tick Size message
                "2|2|9001|0|150|".to_owned(),
                // Tick String message
                "3|2|9001|45|2023-03-13 09:30:00|".to_owned(),
                // Tick Generic message
                "5|2|9001|23|20.5|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL");
        let generic_ticks = &["100", "101", "104", "106"];  // Option Volume, OI, Historical Vol, Implied Vol
        let snapshot = false;
        let regulatory_snapshot = false;

        // Test subscription creation
        let market_data = client.market_data(
            &contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot
        );
        assert!(market_data.is_ok(), "Failed to create market data subscription");

        // Test receiving data
        let market_data = market_data.unwrap();
        let received_ticks: Vec<TickTypes> = market_data.iter().take(4).collect();

        assert_eq!(received_ticks.len(), 4, "Should receive 4 market data updates");

        // Verify different tick types
        for tick in received_ticks {
            match tick {
                TickTypes::Price(tick) => {
                    assert_eq!(tick.tick_type, TickType::Bid, "Wrong tick type");
                    assert_eq!(tick.price, 185.50, "Wrong price");
                    assert_eq!(tick.attributes.can_auto_execute, true, "Wrong can auto execute flag");
                }
                TickTypes::Size(tick) => {
                    assert_eq!(tick.tick_type, TickType::BidSize, "Wrong tick type");
                    assert_eq!(tick.size, 150.0, "Wrong size");
                }
                TickTypes::String(tick) => {
                    assert_eq!(tick.tick_type, TickType::LastTimestamp, "Wrong tick type");
                    assert_eq!(tick.value, "2023-03-13 09:30:00", "Wrong timestamp");
                }
                TickTypes::Generic(tick) => {
                    assert_eq!(tick.tick_type, TickType::OptionHistoricalVol, "Wrong tick type");
                    assert_eq!(tick.value, 20.5, "Wrong value");
                }
                _ => panic!("Unexpected tick type received"),
            }
        }

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(
            request[0],
            OutgoingMessages::RequestMarketData.to_field(),
            "Wrong message type"
        );
        assert_eq!(request[1], "11", "Wrong version");
        assert_eq!(request[17], "100,101,104,106", "Wrong generic ticks");
        assert_eq!(request[18], snapshot.to_field(), "Wrong snapshot flag");
    }

    #[test]
    fn test_market_data_with_combo_legs() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "1|2|9001|1|185.50|100|7|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        
        let mut contract = Contract::stock("AAPL");
        contract.security_type = SecurityType::Spread;
        contract.combo_legs = vec![
            ComboLeg {
                contract_id: 1,
                ratio: 1,
                action: "BUY".to_string(),
                exchange: "SMART".to_string(),
                ..Default::default()
            },
            ComboLeg {
                contract_id: 2,
                ratio: 1,
                action: "SELL".to_string(),
                exchange: "SMART".to_string(),
                ..Default::default()
            },
        ];

        let generic_ticks = &["100"];
        let snapshot = false;
        let regulatory_snapshot = false;

        let market_data = client.market_data(
            &contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot
        ).expect("Failed to create market data subscription");

        // Verify request message contains combo legs
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];

        // Find combo legs section in the message
        let combo_legs_count_index = 15;
        assert_eq!(request[combo_legs_count_index], "2", "Wrong combo legs count");
        
        // Verify first leg
        assert_eq!(request[combo_legs_count_index + 1], "1", "Wrong first leg contract id");
        assert_eq!(request[combo_legs_count_index + 2], "1", "Wrong first leg ratio");
        assert_eq!(request[combo_legs_count_index + 3], "BUY", "Wrong first leg action");
        assert_eq!(request[combo_legs_count_index + 4], "SMART", "Wrong first leg exchange");
        
        // Verify second leg
        assert_eq!(request[combo_legs_count_index + 5], "2", "Wrong second leg contract id");
        assert_eq!(request[combo_legs_count_index + 6], "1", "Wrong second leg ratio");
        assert_eq!(request[combo_legs_count_index + 7], "SELL", "Wrong second leg action");
        assert_eq!(request[combo_legs_count_index + 8], "SMART", "Wrong second leg exchange");
    }

    #[test]
    fn test_market_data_with_delta_neutral() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "1|2|9001|1|185.50|100|7|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        
        let mut contract = Contract::stock("AAPL");
        contract.delta_neutral_contract = Some(DeltaNeutralContract {
            contract_id: 12345,
            delta: 0.5,
            price: 100.0,
        });

        let generic_ticks = &["100"];
        let snapshot = false;
        let regulatory_snapshot = false;

        let market_data = client.market_data(
            &contract,
            generic_ticks,
            snapshot,
            regulatory_snapshot
        ).expect("Failed to create market data subscription");

        // Verify request message contains delta neutral contract
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];

        // Find delta neutral marker in message
        let delta_neutral_index = request.iter()
            .position(|x| x == "true")
            .expect("No delta neutral marker");

        assert_eq!(request[delta_neutral_index + 1], "12345", "Wrong delta neutral contract id");
        assert_eq!(request[delta_neutral_index + 2], "0.5", "Wrong delta");
        assert_eq!(request[delta_neutral_index + 3], "100", "Wrong price");
    }

    #[test]
    fn test_market_data_regulatory_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        // Test with server version that supports regulatory snapshots
        let client = Client::stubbed(message_bus, server_versions::REQ_SMART_COMPONENTS);
        let contract = Contract::stock("AAPL");
        
        let market_data = client.market_data(
            &contract,
            &[],
            false,
            true  // regulatory snapshot
        ).expect("Failed to create market data subscription");

        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];
        assert_eq!(request[19], "true", "Regulatory snapshot flag should be set");

        // Test with older server version
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(message_bus, server_versions::REQ_SMART_COMPONENTS - 1);
        
        let result = client.market_data(
            &contract,
            &[],
            false,
            true  // regulatory snapshot
        );
        
        assert!(result.is_err(), "Should fail with old server version when requesting regulatory snapshot");
    }

    #[test]
    fn test_market_data_error_handling() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "9|2|9001|Error Message|".to_owned(),  // Error message
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL");
        
        let market_data = client.market_data(
            &contract,
            &[],
            false,
            false
        ).expect("Failed to create market data subscription");

        let received_messages: Vec<TickTypes> = market_data.iter().take(1).collect();
        assert_eq!(received_messages.len(), 1, "Should receive error message");

        match &received_messages[0] {
            TickTypes::Notice(notice) => {
                assert_eq!(notice.message, "Error Message", "Wrong error message");
            }
            _ => panic!("Expected error notice"),
        }
    }
}