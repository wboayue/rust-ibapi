use super::*;
use crate::common::test_utils::helpers::{assert_proto_msg_id, count_proto_msgs};
use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::messages::OutgoingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use std::sync::Arc;
use std::sync::RwLock;
use time::macros::datetime;

#[tokio::test]
async fn test_head_timestamp() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["88|9000|1678838400|".to_owned()], // 2023-03-15 00:00:00 UTC
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let result = client.head_timestamp(&contract, what_to_show, trading_hours).await;
    assert!(result.is_ok(), "head_timestamp should succeed");

    let timestamp = result.unwrap();
    assert_eq!(timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHeadTimestamp);
}

#[tokio::test]
async fn test_histogram_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["96|9000|3|185.50|100|185.75|150|186.00|200|".to_owned()],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_HISTOGRAM);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let trading_hours = TradingHours::Regular;
    let period = BarSize::Day;

    let result = client.histogram_data(&contract, trading_hours, period).await;
    assert!(result.is_ok(), "histogram_data should succeed");

    let entries = result.unwrap();
    assert_eq!(entries.len(), 3, "Should receive 3 histogram entries");

    // Verify first entry
    assert_eq!(entries[0].price, 185.50, "Wrong price for first entry");
    assert_eq!(entries[0].size, 100, "Wrong size for first entry");

    // Verify second entry
    assert_eq!(entries[1].price, 185.75, "Wrong price for second entry");
    assert_eq!(entries[1].size, 150, "Wrong size for second entry");

    // Verify third entry
    assert_eq!(entries[2].price, 186.00, "Wrong price for third entry");
    assert_eq!(entries[2].size, 200, "Wrong size for third entry");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistogramData);
}

#[tokio::test]
async fn test_historical_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "17|9000|20230315  09:30:00|20230315  10:30:00|2|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|1678890000|185.75|186.25|185.50|186.00|1500|185.85|150|"
                .to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    // Set client timezone for test
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));
    let duration = Duration::seconds(3600);
    let bar_size = BarSize::Min30;
    let what_to_show = Some(WhatToShow::Trades);
    let trading_hours = TradingHours::Regular;

    let result = client
        .historical_data(&contract, end_date, duration, bar_size, what_to_show, trading_hours)
        .await;
    assert!(result.is_ok(), "historical_data should succeed");

    let data = result.unwrap();
    assert_eq!(data.bars.len(), 2, "Should receive 2 bars");

    // Verify first bar
    let bar = &data.bars[0];
    // 1678886400 = 2023-03-15 13:20:00 UTC
    assert_eq!(bar.date, datetime!(2023-03-15 13:20:00 UTC), "Wrong date for first bar");
    assert_eq!(bar.open, 185.50, "Wrong open for first bar");
    assert_eq!(bar.high, 186.00, "Wrong high for first bar");
    assert_eq!(bar.low, 185.25, "Wrong low for first bar");
    assert_eq!(bar.close, 185.75, "Wrong close for first bar");
    assert_eq!(bar.volume, 1000.0, "Wrong volume for first bar");
    assert_eq!(bar.wap, 185.70, "Wrong WAP for first bar");
    assert_eq!(bar.count, 100, "Wrong count for first bar");

    // Verify second bar
    let bar = &data.bars[1];
    // 1678890000 = 2023-03-15 14:20:00 UTC
    assert_eq!(bar.date, datetime!(2023-03-15 14:20:00 UTC), "Wrong date for second bar");
    assert_eq!(bar.open, 185.75, "Wrong open for second bar");
    assert_eq!(bar.high, 186.25, "Wrong high for second bar");
    assert_eq!(bar.low, 185.50, "Wrong low for second bar");
    assert_eq!(bar.close, 186.00, "Wrong close for second bar");
    assert_eq!(bar.volume, 1500.0, "Wrong volume for second bar");
    assert_eq!(bar.wap, 185.85, "Wrong WAP for second bar");
    assert_eq!(bar.count, 150, "Wrong count for second bar");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[tokio::test]
async fn test_historical_data_version_check() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::TRADING_CLASS - 1);

    let mut contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    contract.trading_class = "ES".to_string(); // Requires TRADING_CLASS version

    let result = client
        .historical_data(&contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular)
        .await;
    assert!(result.is_err(), "Should fail version check");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("trading class"),
        "Error should mention trading class feature: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_historical_data_adjusted_last_validation() {
    let message_bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("AAPL").build();
    let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));

    let result = client
        .historical_data(
            &contract,
            end_date,
            Duration::days(1),
            BarSize::Day,
            Some(WhatToShow::AdjustedLast),
            TradingHours::Regular,
        )
        .await;

    assert!(result.is_err(), "Should fail when end_date is provided with AdjustedLast");
    assert!(
        result.unwrap_err().to_string().contains("end_date must be None"),
        "Error should mention end_date restriction"
    );
}

#[tokio::test]
async fn test_historical_data_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["4|2|9000|162|Historical Market Data Service error message:No market data permissions.|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let result = client
        .historical_data(&contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular)
        .await;
    assert!(result.is_err(), "Should fail with error response");
    assert!(
        result.unwrap_err().to_string().contains("No market data permissions"),
        "Error should contain the error message"
    );
}

#[tokio::test]
async fn test_historical_data_unexpected_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["1|2|9000|1|185.50|100|7|".to_owned()], // Wrong message type
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let result = client
        .historical_data(&contract, None, Duration::days(1), BarSize::Hour, None, TradingHours::Regular)
        .await;
    assert!(result.is_err(), "Should fail with unexpected response");
    matches!(result.unwrap_err(), Error::UnexpectedResponse(_));
}

#[tokio::test]
async fn test_historical_schedule() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "106|9000|20230313-09:30:00|20230315-16:00:00|UTC|3|20230313-09:30:00|20230313-16:00:00|20230313|20230314-09:30:00|20230314-16:00:00|20230314|20230315-09:30:00|20230315-16:00:00|20230315|"
                .to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
    let contract = Contract::stock("AAPL").build();
    let end_date = Some(datetime!(2023-03-15 16:00:00 UTC));
    let duration = Duration::days(3);

    let result = client.historical_schedule(&contract, end_date, duration).await;
    assert!(result.is_ok(), "historical_schedule should succeed");

    let schedule = result.unwrap();
    assert_eq!(schedule.time_zone, "UTC", "Wrong time zone");
    // Check that we have sessions
    assert!(!schedule.sessions.is_empty(), "Should have at least 1 session");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[tokio::test]
async fn test_tick_subscription_methods() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // HistoricalTickBidAsk = 97
            // First response with 2 ticks, not done
            // Format: message_type|request_id|num_ticks|timestamp|mask|bid|ask|bid_size|ask_size|...|done
            "97|9000|2|1678838400|10|185.50|186.00|100|200|1678838401|11|185.55|186.05|105|205|0|".to_owned(),
            // Second response with 1 tick, done
            "97|9000|1|1678838500|10|185.75|186.25|150|250|1|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let mut subscription = client
        .historical_ticks_bid_ask(&contract, None, None, 3, TradingHours::Regular, false)
        .await
        .expect("Failed to create tick subscription");

    // Get first tick
    let tick1 = subscription.next().await;
    assert!(tick1.is_some(), "Should receive first tick");
    let tick1 = tick1.unwrap();
    assert_eq!(tick1.price_bid, 185.50, "Wrong bid price for first tick");
    assert_eq!(tick1.price_ask, 186.00, "Wrong ask price for first tick");
    assert_eq!(tick1.size_bid, 100, "Wrong bid size for first tick");
    assert_eq!(tick1.size_ask, 200, "Wrong ask size for first tick");
    assert!(tick1.tick_attribute_bid_ask.bid_past_low, "Wrong bid past low for first tick");
    assert!(!tick1.tick_attribute_bid_ask.ask_past_high, "Wrong ask past high for first tick");

    // Get second tick
    let tick2 = subscription.next().await;
    assert!(tick2.is_some(), "Should receive second tick");
    let tick2 = tick2.unwrap();
    assert_eq!(tick2.price_bid, 185.55, "Wrong bid price for second tick");
    assert_eq!(tick2.price_ask, 186.05, "Wrong ask price for second tick");

    // Get third tick
    let tick3 = subscription.next().await;
    assert!(tick3.is_some(), "Should receive third tick");
    let tick3 = tick3.unwrap();
    assert_eq!(tick3.price_bid, 185.75, "Wrong bid price for third tick");

    // Should be done now
    let tick4 = subscription.next().await;
    assert!(tick4.is_none(), "Should not receive more ticks after done");
}

#[tokio::test]
async fn test_tick_subscription_buffer_and_iteration() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // HistoricalTickBidAsk = 97
            // Response with 3 ticks at once, done = true
            "97|9000|3|1678838400|8|185.50|186.00|100|200|1678838401|9|185.60|186.10|110|210|1678838402|10|185.70|186.20|120|220|1|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let mut subscription = client
        .historical_ticks_bid_ask(&contract, None, None, 3, TradingHours::Regular, false)
        .await
        .expect("Failed to create tick subscription");

    // Should receive all 3 ticks from buffer
    let mut ticks = Vec::new();
    while let Some(tick) = subscription.next().await {
        ticks.push(tick);
    }

    assert_eq!(ticks.len(), 3, "Should receive exactly 3 ticks");
    assert_eq!(ticks[0].price_bid, 185.50, "Wrong bid price for first tick");
    assert_eq!(ticks[1].price_bid, 185.60, "Wrong bid price for second tick");
    assert_eq!(ticks[2].price_bid, 185.70, "Wrong bid price for third tick");
}

#[tokio::test]
async fn test_tick_subscription_bid_ask() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // HistoricalTickBidAsk = 97
            // mask = 2 (binary 10) = bid_past_low = true, ask_past_high = false
            "97|9000|1|1678838400|2|185.50|186.00|100|200|1|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let start = Some(datetime!(2023-03-15 09:00:00 UTC));
    let end = Some(datetime!(2023-03-15 10:00:00 UTC));
    let number_of_ticks = 1;
    let trading_hours = TradingHours::Regular;
    let ignore_size = false;

    let mut subscription = client
        .historical_ticks_bid_ask(&contract, start, end, number_of_ticks, trading_hours, ignore_size)
        .await
        .expect("Failed to create bid/ask tick subscription");

    let tick = subscription.next().await.expect("Should receive a tick");
    assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
    assert_eq!(tick.price_bid, 185.50, "Wrong bid price");
    assert_eq!(tick.price_ask, 186.00, "Wrong ask price");
    assert_eq!(tick.size_bid, 100, "Wrong bid size");
    assert_eq!(tick.size_ask, 200, "Wrong ask size");
    assert!(tick.tick_attribute_bid_ask.bid_past_low, "Wrong bid past low");
    assert!(!tick.tick_attribute_bid_ask.ask_past_high, "Wrong ask past high");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");

    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalTicks);
}

#[tokio::test]
async fn test_tick_subscription_midpoint() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // HistoricalTick = 96 (for midpoint)
            // Format: message_type|request_id|num_ticks|timestamp|skip|price|size|...|done
            "96|9000|1|1678838400|0|185.75|100|1|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let mut subscription = client
        .historical_ticks_mid_point(&contract, None, None, 1, TradingHours::Regular)
        .await
        .expect("Failed to create midpoint tick subscription");

    let tick = subscription.next().await.expect("Should receive a tick");
    assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
    assert_eq!(tick.price, 185.75, "Wrong midpoint price");
    assert_eq!(tick.size, 100, "Wrong size");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalTicks);
}

#[tokio::test]
async fn test_historical_ticks_trade() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // HistoricalTickLast = 98
            // Format: message_type|request_id|num_ticks|timestamp|mask|price|size|exchange|conditions|...|done
            "98|9000|1|1678838400|0|185.50|100|ISLAND|APR|1|".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::HISTORICAL_TICKS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let mut subscription = client
        .historical_ticks_trade(&contract, None, None, 1, TradingHours::Regular)
        .await
        .expect("Failed to create trade tick subscription");

    let tick = subscription.next().await.expect("Should receive a tick");
    assert_eq!(tick.timestamp, datetime!(2023-03-15 00:00:00 UTC), "Wrong timestamp");
    assert_eq!(tick.price, 185.50, "Wrong trade price");
    assert_eq!(tick.size, 100, "Wrong trade size");
    assert_eq!(tick.exchange, "ISLAND", "Wrong exchange");
    assert_eq!(tick.special_conditions, "APR", "Wrong special conditions");
    assert!(!tick.tick_attribute_last.past_limit, "Wrong past limit");
    assert!(!tick.tick_attribute_last.unreported, "Wrong unreported");

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request message");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalTicks);
}

#[tokio::test]
async fn test_historical_data_time_zone_handling() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned()],
    });

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    // Set client timezone to Eastern
    client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let result = client
        .historical_data(&contract, None, Duration::seconds(3600), BarSize::Hour, None, TradingHours::Regular)
        .await;

    assert!(result.is_ok(), "historical_data should succeed with timezone");
    let data = result.unwrap();
    assert_eq!(data.bars.len(), 1, "Should receive 1 bar");

    // The timestamp should be parsed in the client's timezone
    // 1678886400 = 2023-03-15 12:00:00 UTC = 2023-03-15 08:00:00 EDT
    let bar = &data.bars[0];
    assert_eq!(bar.date.unix_timestamp(), 1678886400, "Timestamp should match");
}

#[tokio::test]
async fn test_time_zone_fallback() {
    let mut client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::SIZE_RULES);
    // Client without timezone set
    client.time_zone = None;

    let tz = time_zone(&client);
    assert_eq!(tz, time_tz::timezones::db::UTC, "Should fallback to UTC when timezone not set");
}

#[tokio::test]
async fn test_historical_data_streaming_with_updates() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Initial historical data (message type 17)
            "17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned(),
            // Streaming update (message type 90)
            "90|9000|-1|1678890000|185.80|186.10|185.60|185.90|500|185.85|50|".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .await
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let update1 = subscription.next().await.expect("Should receive initial historical data");
    match update1.expect("decode should succeed") {
        HistoricalBarUpdate::Historical(data) => {
            assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
            assert_eq!(data.bars[0].open, 185.50, "Wrong open price");
        }
        _ => panic!("Expected Historical variant"),
    }

    // Second: receive streaming update
    let update2 = subscription.next().await.expect("Should receive streaming update");
    match update2.expect("decode should succeed") {
        HistoricalBarUpdate::Update(bar) => {
            assert_eq!(bar.open, 185.80, "Wrong open price in update");
            assert_eq!(bar.high, 186.10, "Wrong high price in update");
            assert_eq!(bar.close, 185.90, "Wrong close price in update");
        }
        _ => panic!("Expected Update variant"),
    }

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[tokio::test]
async fn test_historical_data_streaming_keep_up_to_date_false() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Initial historical data only
            "17|9000|20230315  09:30:00|20230315  10:30:00|1|1678886400|185.50|186.00|185.25|185.75|1000|185.70|100|".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            false, // keep_up_to_date = false
        )
        .await
        .expect("streaming request should succeed");

    // Receive initial historical data
    let update1 = subscription.next().await.expect("Should receive initial historical data");
    match update1.expect("decode should succeed") {
        HistoricalBarUpdate::Historical(data) => {
            assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
        }
        _ => panic!("Expected Historical variant"),
    }

    // Verify request message
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[tokio::test]
async fn test_historical_data_streaming_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Error response
            "4|2|9000|162|Historical Market Data Service error message:No market data permissions.|".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let mut subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .await
        .expect("streaming request should succeed");

    // Should yield Some(Err(_)) — Subscription<T> surfaces errors through next().
    let update = subscription.next().await.expect("error should arrive as Some(Err(_))");
    let err = update.expect_err("Should yield error result");
    assert!(err.to_string().contains("No market data permissions"), "Error should contain the message");
}

#[tokio::test]
async fn test_tick_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let _subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9100, message_bus.clone());
        // dropped here, !done so cancel should fire
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 1, "should send cancel message on drop");
    assert_proto_msg_id(&messages[0], OutgoingMessages::CancelHistoricalTicks);
}

#[tokio::test]
async fn test_tick_subscription_explicit_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9101, message_bus.clone());
        subscription.cancel().await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 1, "should send cancel only once");
}

#[tokio::test]
async fn test_tick_subscription_drop_after_done_does_not_cancel() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let (_tx, rx) = tokio::sync::broadcast::channel(16);
    let internal = AsyncInternalSubscription::new(rx);

    {
        let mut subscription: TickSubscription<TickLast> = TickSubscription::new(internal, 9102, message_bus.clone());
        subscription.done = true;
        // drop with done=true → no cancel
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(messages.len(), 0, "completed subscription should not send cancel on drop");
}

#[tokio::test]
async fn test_streaming_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let _subscription = client
            .historical_data_streaming(
                &contract,
                Duration::days(1),
                BarSize::Hour,
                Some(WhatToShow::Trades),
                TradingHours::Regular,
                true,
            )
            .await
            .expect("streaming request should succeed");
        // subscription dropped here
    }

    // Give tokio::spawn time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send exactly one cancel message on drop"
    );
}

#[tokio::test]
async fn test_streaming_subscription_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);
    let contract = Contract::stock("SPY").build();

    {
        let subscription = client
            .historical_data_streaming(
                &contract,
                Duration::days(1),
                BarSize::Hour,
                Some(WhatToShow::Trades),
                TradingHours::Regular,
                true,
            )
            .await
            .expect("streaming request should succeed");

        // Explicit cancel; drop should not fire a second cancel.
        subscription.cancel().await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = message_bus.request_messages.read().unwrap();
    assert_eq!(
        count_proto_msgs(&messages, OutgoingMessages::CancelHistoricalData),
        1,
        "should send cancel only once"
    );
}
