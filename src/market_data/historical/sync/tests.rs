use super::*;
use crate::client::blocking::Client;
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::contracts::Contract;
use crate::market_data::historical::ToDuration;
use crate::market_data::TradingHours;
use crate::messages::OutgoingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use std::sync::{Arc, RwLock};
use time::macros::{date, datetime};
use time::OffsetDateTime;
use time_tz::{self, PrimitiveDateTimeExt, Tz};

#[test]
fn test_head_timestamp() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["88|9000|1678323335|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("MSFT").build();
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let head_timestamp = client
        .head_timestamp(&contract, what_to_show, trading_hours)
        .expect("head timestamp request failed");

    assert_eq!(head_timestamp, OffsetDateTime::from_unix_timestamp(1678323335).unwrap(), "bar.date");

    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHeadTimestamp);
}

#[test]
fn test_histogram_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["19|9000|3|125.50|1000|126.00|2000|126.50|3000|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("MSFT").build();
    let trading_hours = TradingHours::Regular;
    let period = BarSize::Day;

    let histogram_data = client
        .histogram_data(&contract, trading_hours, period)
        .expect("histogram data request failed");

    // Assert Response
    assert_eq!(histogram_data.len(), 3, "histogram_data.len()");

    assert_eq!(histogram_data[0].price, 125.50, "histogram_data[0].price");
    assert_eq!(histogram_data[0].size, 1000, "histogram_data[0].size");

    assert_eq!(histogram_data[1].price, 126.00, "histogram_data[1].price");
    assert_eq!(histogram_data[1].size, 2000, "histogram_data[1].size");

    assert_eq!(histogram_data[2].price, 126.50, "histogram_data[2].price");
    assert_eq!(histogram_data[2].size, 3000, "histogram_data[2].size");

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert!(!request_messages.is_empty(), "Should have sent a request message");
}

#[test]
fn test_historical_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "17|9000|20230413  16:31:22|20230415  16:31:22|2|20230413|182.9400|186.5000|180.9400|185.9000|948837.22|184.869|324891|20230414|183.8800|186.2800|182.0100|185.0000|810998.27|183.9865|277547|".to_owned()
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("MSFT").build();
    let interval_end = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let historical_data = client
        .historical_data(&contract, Some(interval_end), duration, bar_size, what_to_show, trading_hours)
        .expect("historical data request failed");

    // Assert Response

    assert_eq!(historical_data.start, datetime!(2023-04-13 16:31:22 UTC), "historical_data.start");
    assert_eq!(historical_data.end, datetime!(2023-04-15 16:31:22 UTC), "historical_data.end");
    assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");

    assert_eq!(historical_data.bars[0].date, datetime!(2023-04-13 00:00:00 UTC), "bar.date");
    assert_eq!(historical_data.bars[0].open, 182.94, "bar.open");
    assert_eq!(historical_data.bars[0].high, 186.50, "bar.high");
    assert_eq!(historical_data.bars[0].low, 180.94, "bar.low");
    assert_eq!(historical_data.bars[0].close, 185.90, "bar.close");
    assert_eq!(historical_data.bars[0].volume, 948837.22, "bar.volume");
    assert_eq!(historical_data.bars[0].wap, 184.869, "bar.wap");
    assert_eq!(historical_data.bars[0].count, 324891, "bar.count");

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[test]
fn test_historical_schedule() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_SCHEDULE);

    let contract = Contract::stock("MSFT").build();
    let end_date = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 7.days();

    let schedule = client
        .historical_schedules(&contract, end_date, duration)
        .expect("historical schedule request failed");

    // Assert Response
    assert_eq!(schedule.time_zone, "US/Eastern", "schedule.time_zone");

    let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;
    assert_eq!(
        schedule.start,
        datetime!(2023-04-14 09:30:00).assume_timezone(time_zone).unwrap(),
        "schedule.start"
    );
    assert_eq!(
        schedule.end,
        datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
        "schedule.end"
    );

    assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
    assert_eq!(schedule.sessions[0].reference, date!(2023 - 04 - 14), "schedule.sessions[0].reference");
    assert_eq!(
        schedule.sessions[0].start,
        datetime!(2023-04-14 09:30:00).assume_timezone(time_zone).unwrap(),
        "schedule.sessions[0].start"
    );
    assert_eq!(
        schedule.sessions[0].end,
        datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
        "schedule.sessions[0].end"
    );

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages.len(), 1, "Should send one request message");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[test]
fn test_historical_ticks_bid_ask() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = Some(datetime!(2023-04-01 09:30:00 UTC));
    let end = Some(datetime!(2023-04-01 16:00:00 UTC));
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;
    let ignore_size = true;

    // Just test that the function doesn't panic and returns a subscription
    let _tick_subscription = client
        .historical_ticks_bid_ask(&contract, start, end, number_of_ticks, trading_hours, ignore_size)
        .expect("historical ticks bid ask request failed");

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert!(!request_messages.is_empty(), "Should have sent a request message");
}

#[test]
fn test_historical_ticks_mid_point() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = Some(datetime!(2023-04-01 09:30:00 UTC));
    let end = Some(datetime!(2023-04-01 16:00:00 UTC));
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    // Just test that the function doesn't panic and returns a subscription
    let _tick_subscription = client
        .historical_ticks_mid_point(&contract, start, end, number_of_ticks, trading_hours)
        .expect("historical ticks mid point request failed");

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert!(!request_messages.is_empty(), "Should have sent a request message");
}

#[test]
fn test_historical_ticks_trade() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let start = Some(datetime!(2023-04-01 09:30:00 UTC));
    let end = Some(datetime!(2023-04-01 16:00:00 UTC));
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    // Just test that the function doesn't panic and returns a subscription
    let _tick_subscription = client
        .historical_ticks_trade(&contract, start, end, number_of_ticks, trading_hours)
        .expect("historical ticks trade request failed");

    // Assert Request
    let request_messages = client.message_bus.request_messages();
    assert!(!request_messages.is_empty(), "Should have sent a request message");
}

#[test]
fn test_historical_data_version_check() {
    // Test with a server version that doesn't support trading class
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    // Use an older server version
    let client = Client::stubbed(message_bus, server_versions::TRADING_CLASS - 1);

    // Create a contract with trading_class set
    let mut contract = Contract::stock("MSFT").build();
    contract.trading_class = "CLASS".to_string();

    let end_date = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let trading_hours = TradingHours::Regular;

    // This should return an error due to server version
    let result = client.historical_data(&contract, Some(end_date), duration, bar_size, WhatToShow::Trades, trading_hours);
    assert!(result.is_err(), "Expected error due to server version incompatibility");
}

#[test]
fn test_historical_data_adjusted_last_validation() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let end_date = Some(datetime!(2023-04-15 16:31:22 UTC));
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::AdjustedLast;
    let trading_hours = TradingHours::Regular;

    // This should return an error because AdjustedLast can't be used with end_date
    let result = client.historical_data(&contract, end_date, duration, bar_size, what_to_show, trading_hours);
    assert!(result.is_err(), "Expected error due to AdjustedLast with end_date");

    match result {
        Err(Error::InvalidArgument(_)) => {
            // This is the expected error type
        }
        _ => panic!("Expected InvalidArgument error"),
    }
}

#[test]
fn test_historical_data_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Respond with an error message
            "3\09000\0200\0No security definition has been found for the request\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    // This should return an error because the server sent an error response
    let result = client.historical_data(&contract, None, duration, bar_size, what_to_show, trading_hours);
    assert!(result.is_err(), "Expected error due to error response from server");
}

#[test]
fn test_historical_data_unexpected_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Respond with an unexpected message type (using market data type message)
            "58\09000\02\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("MSFT").build();
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    // This should return an error because the server sent an unexpected response
    let result = client.historical_data(&contract, None, duration, bar_size, what_to_show, trading_hours);
    assert!(result.is_err(), "Expected error due to unexpected response type");

    match result {
        Err(Error::UnexpectedResponse(_)) => {
            // This is the expected error type
        }
        _ => panic!("Expected UnexpectedResponse error"),
    }
}

#[test]
fn test_tick_subscription_methods() {
    // For now, we'll use a minimal test to ensure the methods exist and are called correctly
    // Testing the subscription iterators fully would require more complex setup with mocked messages

    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let tick_subscription = client
        .historical_ticks_trade(&contract, None, None, number_of_ticks, trading_hours)
        .expect("historical ticks trade request failed");

    // Just test that these methods can be called without panicking
    let _iter = tick_subscription.iter();
    let _try_iter = tick_subscription.try_iter();
    let _timeout_iter = tick_subscription.timeout_iter(std::time::Duration::from_millis(100));

    // Test IntoIterator trait exists
    let _iter_ref: TickSubscriptionIter<TickLast> = (&tick_subscription).into_iter();
}

#[test]
fn test_tick_subscription_buffer_and_iteration() {
    // Create a message bus with predetermined responses
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // First response has 3 ticks, done = false
            "98\09000\03\01681133400\00\011.63\024547\0ISLAND\0 O X\01681133401\00\011.64\0179\0FINRA\0\01681133402\00\011.65\0200\0NYSE\0\00\0"
                .to_owned(),
            // Second response has 2 ticks, done = true
            "98\09000\02\01681133403\00\011.66\0100\0ARCA\0\01681133404\00\011.67\0300\0BATS\0\01\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let number_of_ticks = 10;
    let trading_hours = TradingHours::Regular;

    let tick_subscription = client
        .historical_ticks_trade(&contract, None, None, number_of_ticks, trading_hours)
        .expect("historical ticks trade request failed");

    // Test standard iterator
    let mut ticks = Vec::new();
    for tick in tick_subscription.iter() {
        ticks.push(tick);
    }

    // Should have received all 5 ticks from both messages
    assert_eq!(ticks.len(), 5, "Expected 5 ticks in total");

    // Check specific values from first and last ticks
    assert_eq!(ticks[0].price, 11.63, "First tick price");
    assert_eq!(ticks[0].exchange, "ISLAND", "First tick exchange");

    assert_eq!(ticks[4].price, 11.67, "Last tick price");
    assert_eq!(ticks[4].exchange, "BATS", "Last tick exchange");
}

#[test]
fn test_tick_subscription_owned_iterator() {
    // Test that the owned iterator works correctly
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["98\09000\02\01681133400\00\011.70\024547\0ISLAND\0 O X\01681133401\00\011.71\0179\0FINRA\0\01\0".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks_trade(&contract, None, None, 10, TradingHours::Regular)
        .expect("historical ticks trade request failed");

    // Convert to owned iterator
    let ticks: Vec<TickLast> = tick_subscription.into_iter().collect();

    assert_eq!(ticks.len(), 2, "Expected 2 ticks from owned iterator");
    assert_eq!(ticks[0].price, 11.70, "First tick price");
    assert_eq!(ticks[1].price, 11.71, "Second tick price");
}

#[test]
fn test_tick_subscription_bid_ask() {
    // Create a message bus with bid/ask tick data
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "97\09000\03\01681133399\00\011.63\011.83\02800\0100\01681133400\00\011.64\011.84\02900\0200\01681133401\00\011.65\011.85\03000\0300\01\0".to_owned(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks_bid_ask(&contract, None, None, 10, TradingHours::Regular, false)
        .expect("historical ticks bid_ask request failed");

    // Collect ticks
    let ticks: Vec<TickBidAsk> = tick_subscription.iter().collect();

    assert_eq!(ticks.len(), 3, "Expected 3 bid/ask ticks");

    // Check first tick
    assert_eq!(ticks[0].price_bid, 11.63, "First tick bid price");
    assert_eq!(ticks[0].price_ask, 11.83, "First tick ask price");
    assert_eq!(ticks[0].size_bid, 2800, "First tick bid size");
    assert_eq!(ticks[0].size_ask, 100, "First tick ask size");

    // Check last tick
    assert_eq!(ticks[2].price_bid, 11.65, "Last tick bid price");
    assert_eq!(ticks[2].price_ask, 11.85, "Last tick ask price");
}

#[test]
fn test_tick_subscription_midpoint() {
    // Create a message bus with midpoint tick data
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["96\09000\03\01681133398\00\091.36\00\01681133399\00\091.37\00\01681133400\00\091.38\00\01\0".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::HISTORICAL_TICKS);

    let contract = Contract::stock("MSFT").build();
    let tick_subscription = client
        .historical_ticks_mid_point(&contract, None, None, 10, TradingHours::Regular)
        .expect("historical ticks mid_point request failed");

    // Collect ticks
    let ticks: Vec<TickMidpoint> = tick_subscription.iter().collect();

    assert_eq!(ticks.len(), 3, "Expected 3 midpoint ticks");

    // Check specific tick values
    assert_eq!(ticks[0].price, 91.36, "First tick price");
    assert_eq!(ticks[1].price, 91.37, "Second tick price");
    assert_eq!(ticks[2].price, 91.38, "Third tick price");
}

#[test]
fn test_historical_data_time_zone_handling() {
    // Test with explicit Eastern time zone data
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Format: historical data with NY timezone in the response
            "17\09000\020230413  09:30:00\020230415  16:00:00\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0".to_owned()
        ],
    });

    // Create a client with a time zone specifically set to NY
    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

    let contract = Contract::stock("MSFT").build();
    let interval_end = datetime!(2023-04-15 16:00:00 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Day;
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    let historical_data = client
        .historical_data(&contract, Some(interval_end), duration, bar_size, what_to_show, trading_hours)
        .expect("historical data request failed");

    // Assert that time zones are correctly handled
    let ny_zone = time_tz::timezones::db::america::NEW_YORK;

    // Start time should be 9:30 AM ET
    assert_eq!(
        historical_data.start,
        datetime!(2023-04-13 09:30:00).assume_timezone(ny_zone).unwrap(),
        "historical_data.start should be in NY timezone"
    );

    // End time should be 4:00 PM ET
    assert_eq!(
        historical_data.end,
        datetime!(2023-04-15 16:00:00).assume_timezone(ny_zone).unwrap(),
        "historical_data.end should be in NY timezone"
    );
}

#[test]
fn test_time_zone_fallback() {
    // Test the time_zone function's fallback behavior
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    // Create a client without a time zone (should fall back to UTC)
    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    // Test that the function returns UTC when client.time_zone is None
    assert_eq!(
        time_zone(&client),
        time_tz::timezones::db::UTC,
        "time_zone should fall back to UTC when client.time_zone is None"
    );

    // Create a client with a time zone set to NY
    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::america::NEW_YORK);

    // Test that the function returns the client's time zone when it is set
    assert_eq!(
        time_zone(&client),
        time_tz::timezones::db::america::NEW_YORK,
        "time_zone should return the client's time zone when it is set"
    );
}

#[test]
fn test_historical_data_streaming_with_updates() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Initial historical data (message type 17)
            "17\09000\020230315  09:30:00\020230315  10:30:00\01\01678886400\0185.50\0186.00\0185.25\0185.75\01000\0185.70\0100\0".to_owned(),
            // Streaming update (message type 90)
            "90\09000\0-1\01678890000\0185.80\0186.10\0185.60\0185.90\0500\0185.85\050\0".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .expect("streaming request should succeed");

    // First: receive initial historical data
    let update1 = subscription.next();
    assert!(update1.is_some(), "Should receive initial historical data");
    match update1.unwrap() {
        HistoricalBarUpdate::Historical(data) => {
            assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
            assert_eq!(data.bars[0].open, 185.50, "Wrong open price");
        }
        _ => panic!("Expected Historical variant"),
    }

    // Second: receive streaming update
    let update2 = subscription.next();
    assert!(update2.is_some(), "Should receive streaming update");
    match update2.unwrap() {
        HistoricalBarUpdate::Update(bar) => {
            assert_eq!(bar.open, 185.80, "Wrong open price in update");
            assert_eq!(bar.high, 186.10, "Wrong high price in update");
            assert_eq!(bar.close, 185.90, "Wrong close price in update");
        }
        _ => panic!("Expected Update variant"),
    }

    // Verify request message was sent
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[test]
fn test_historical_data_streaming_keep_up_to_date_false() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Initial historical data only
            "17\09000\020230315  09:30:00\020230315  10:30:00\01\01678886400\0185.50\0186.00\0185.25\0185.75\01000\0185.70\0100\0".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            false, // keep_up_to_date = false
        )
        .expect("streaming request should succeed");

    // Receive initial historical data
    let update1 = subscription.next();
    assert!(update1.is_some(), "Should receive initial historical data");
    match update1.unwrap() {
        HistoricalBarUpdate::Historical(data) => {
            assert_eq!(data.bars.len(), 1, "Should have 1 initial bar");
        }
        _ => panic!("Expected Historical variant"),
    }

    // Verify request message was sent
    let request_messages = message_bus.request_messages.read().unwrap();
    assert_eq!(request_messages.len(), 1, "Should send one request");
    assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestHistoricalData);
}

#[test]
fn test_historical_data_streaming_error_response() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Error response
            "4\02\09000\0162\0Historical Market Data Service error message:No market data permissions.\0".to_owned(),
        ],
    });

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    client.time_zone = Some(time_tz::timezones::db::UTC);

    let contract = Contract::stock("SPY").build();

    let subscription = client
        .historical_data_streaming(
            &contract,
            Duration::days(1),
            BarSize::Hour,
            Some(WhatToShow::Trades),
            TradingHours::Regular,
            true,
        )
        .expect("streaming request should succeed");

    // Should return None due to error
    let update = subscription.next();
    assert!(update.is_none(), "Should return None on error");

    // Error should be accessible
    let error = subscription.error();
    assert!(error.is_some(), "Error should be stored");
    assert!(
        error.unwrap().to_string().contains("No market data permissions"),
        "Error should contain the message"
    );
}

#[test]
fn test_streaming_subscription_sends_cancel_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let internal = message_bus.send_request(9000, &[]).unwrap();

    {
        let _subscription = HistoricalDataStreamingSubscription::new(
            internal,
            server_versions::SIZE_RULES,
            time_tz::timezones::db::UTC,
            9000,
            message_bus.clone(),
        );
        // subscription dropped here
    }

    let messages = message_bus.request_messages.read().unwrap();
    // First message is the send_request call, second is the cancel
    let cancel_msg = messages.last().expect("should have cancel message");
    assert_proto_msg_id(cancel_msg, OutgoingMessages::CancelHistoricalData);
}

#[test]
fn test_streaming_subscription_cancel_prevents_duplicate_on_drop() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    let internal = message_bus.send_request(9001, &[]).unwrap();

    {
        let subscription = HistoricalDataStreamingSubscription::new(
            internal,
            server_versions::SIZE_RULES,
            time_tz::timezones::db::UTC,
            9001,
            message_bus.clone(),
        );

        // Explicit cancel
        subscription.cancel();

        // Drop should not send a second cancel
    }

    let messages = message_bus.request_messages.read().unwrap();
    let cancel_count = messages
        .iter()
        .filter(|m| {
            if m.len() >= 4 {
                let msg_id = i32::from_be_bytes([m[0], m[1], m[2], m[3]]);
                msg_id == OutgoingMessages::CancelHistoricalData as i32 + 200
            } else {
                false
            }
        })
        .count();
    assert_eq!(cancel_count, 1, "should send cancel only once");
}
