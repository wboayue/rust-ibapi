use super::*;
use crate::common::test_utils::helpers::{assert_request, proto_response, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::tick_types::TickType;
use crate::contracts::{ComboLeg, Contract, Currency, DeltaNeutralContract, Exchange, LegAction, SecurityType, Symbol};
use crate::market_data::realtime::{BidAsk, MidPoint, Trade};
use crate::market_data::{IgnoreSize, SmartDepth, TradingHours};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::market_data::{
    bid_ask_tick, market_data_request, market_depth_exchanges_request, market_depth_request, market_depth_response, mid_point_tick,
    realtime_bar_tick, realtime_bars_request, tick_by_tick_request, tick_generic, tick_price, tick_size, tick_string, trade_tick,
};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;
use std::sync::RwLock;
use time::OffsetDateTime;

#[test]
fn test_validate_tick_by_tick_request() {
    // Test with old server version
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK - 1);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };

    let result = validate_tick_by_tick_request(&client, &contract, 0, false);
    assert!(result.is_err(), "Should fail with old server version");

    // Test with new server version but old parameters
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK);

    let result = validate_tick_by_tick_request(&client, &contract, 1, true);
    assert!(result.is_err(), "Should fail with new server version but old parameters");

    // Test with new server version and new parameters
    let client = Client::stubbed(Arc::new(MessageBusStub::default()), server_versions::TICK_BY_TICK_IGNORE_SIZE);

    let result = validate_tick_by_tick_request(&client, &contract, 1, true);
    assert!(result.is_ok(), "Should succeed with new server version and parameters");
}

#[test]
fn test_realtime_bars() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::RealTimeBars,
            realtime_bar_tick()
                .time(1678323335)
                .ohlc(4028.75, 4029.00, 4028.25, 4028.50)
                .volume(2.0)
                .wap(4026.75)
                .count(1)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::RealTimeBars,
            realtime_bar_tick()
                .time(1678323340)
                .ohlc(4028.80, 4029.10, 4028.30, 4028.55)
                .volume(3.0)
                .wap(4026.80)
                .count(2)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    let contract = Contract {
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        local_symbol: "FGBL MAR 23".to_owned(),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let what_to_show = WhatToShow::Trades;
    let trading_hours = TradingHours::Regular;

    // Test subscription creation
    let bars = client
        .realtime_bars(&contract)
        .what_to_show(what_to_show)
        .trading_hours(trading_hours)
        .subscribe();

    // Test receiving data
    let bars = bars.expect("Failed to create realtime bars subscription");
    let received_bars: Vec<Bar> = bars.iter_data().take(2).map(|r| r.expect("subscription error")).collect();

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

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &realtime_bars_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .what_to_show(what_to_show)
            .use_rth(trading_hours.use_rth()),
    );
}

#[test]
fn test_tick_by_tick_all_last() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::TickByTick,
            trade_tick()
                .tick_type(1)
                .time(1678740829)
                .price(3895.25)
                .size(7.0)
                .attributes(false, true)
                .exchange("NASDAQ")
                .special_conditions("Regular")
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::TickByTick,
            trade_tick()
                .tick_type(1)
                .time(1678740830)
                .price(3895.50)
                .size(5.0)
                .exchange("NYSE")
                .special_conditions("Regular")
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let number_of_ticks = 2;

    // Test subscription creation
    let trades = client.tick_by_tick(&contract, number_of_ticks).all_last();
    let trades = trades.expect("Failed to create tick-by-tick subscription");

    // Test receiving data
    let received_trades: Vec<Trade> = trades.iter_data().take(2).map(|r| r.expect("subscription error")).collect();

    assert_eq!(received_trades.len(), 2, "Should receive 2 trades");

    // Verify first trade
    let trade = &received_trades[0];
    assert_eq!(trade.price, 3895.25, "Wrong price for first trade");
    assert_eq!(trade.size, 7.0, "Wrong size for first trade");
    assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange for first trade");

    // Verify second trade
    let trade = &received_trades[1];
    assert_eq!(trade.price, 3895.50, "Wrong price for second trade");
    assert_eq!(trade.size, 5.0, "Wrong size for second trade");
    assert_eq!(trade.exchange, "NYSE", "Wrong exchange for second trade");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &tick_by_tick_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .tick_type("AllLast")
            .number_of_ticks(number_of_ticks)
            .ignore_size(false),
    );
}

#[test]
fn test_market_depth() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::MarketDepth,
            market_depth_response()
                .position(0)
                .operation(0)
                .side(0)
                .price(4028.75)
                .size(100.0)
                .encode_proto(),
        ),
        proto_response(
            IncomingMessages::MarketDepth,
            market_depth_response()
                .position(1)
                .operation(1)
                .side(1)
                .price(4028.50)
                .size(200.0)
                .encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SMART_DEPTH);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let number_of_rows = 10;
    let smart_depth = SmartDepth::No;

    // Test subscription creation
    let depth = client.market_depth(&contract, number_of_rows).smart_depth(smart_depth).subscribe();
    let depth = depth.expect("Failed to create market depth subscription");

    // Test receiving data
    let received_depth: Vec<MarketDepths> = depth.iter_data().take(2).map(|r| r.expect("subscription error")).collect();

    assert_eq!(received_depth.len(), 2, "Should receive 2 depth updates");

    // Verify first update (insert bid)
    if let MarketDepths::MarketDepth(depth) = &received_depth[0] {
        assert_eq!(depth.position, 0, "Wrong position for first update");
        assert_eq!(depth.operation, 0, "Wrong operation for first update");
        assert_eq!(depth.side, 0, "Wrong side for first update");
        assert_eq!(depth.price, 4028.75, "Wrong price for first update");
        assert_eq!(depth.size, 100.0, "Wrong size for first update");
    } else {
        panic!("Expected MarketDepth, got {:?}", received_depth[0]);
    }

    // Verify second update (update ask)
    if let MarketDepths::MarketDepth(depth) = &received_depth[1] {
        assert_eq!(depth.position, 1, "Wrong position for second update");
        assert_eq!(depth.operation, 1, "Wrong operation for second update");
        assert_eq!(depth.side, 1, "Wrong side for second update");
        assert_eq!(depth.price, 4028.50, "Wrong price for second update");
        assert_eq!(depth.size, 200.0, "Wrong size for second update");
    } else {
        panic!("Expected MarketDepth, got {:?}", received_depth[1]);
    }

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &market_depth_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .number_of_rows(number_of_rows)
            .smart_depth(smart_depth.is_enabled()),
    );
}

#[test]
fn test_market_depth_exchanges() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["71|2|ISLAND|STK|NASDAQ|DEEP2|1|NYSE|STK|NYSE|DEEP|1|".to_owned()],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::SERVICE_DATA_TYPE);

    // Test request execution
    let exchanges = client.market_depth_exchanges().expect("Failed to request market depth exchanges");

    assert_eq!(exchanges.len(), 2, "Should receive 2 exchange descriptions");

    // Verify first exchange
    let first = &exchanges[0];
    assert_eq!(first.exchange_name, "ISLAND", "Wrong exchange name");
    assert_eq!(first.security_type, "STK", "Wrong security type");
    assert_eq!(first.listing_exchange, "NASDAQ", "Wrong listing exchange");
    assert_eq!(first.service_data_type, "DEEP2", "Wrong service data type");
    assert_eq!(first.aggregated_group, Some("1".to_string()), "Wrong aggregated group");

    // Verify second exchange
    let second = &exchanges[1];
    assert_eq!(second.exchange_name, "NYSE", "Wrong exchange name");
    assert_eq!(second.security_type, "STK", "Wrong security type");
    assert_eq!(second.listing_exchange, "NYSE", "Wrong listing exchange");
    assert_eq!(second.service_data_type, "DEEP", "Wrong service data type");
    assert_eq!(second.aggregated_group, Some("1".to_string()), "Wrong aggregated group");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &market_depth_exchanges_request());
}

#[test]
fn test_tick_by_tick_bid_ask() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::TickByTick,
        bid_ask_tick()
            .time(1678745793)
            .quote(3895.50, 3896.00, 9.0, 11.0)
            .attributes(true, true)
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let number_of_ticks = 1;

    // Test subscription creation
    let result = client.tick_by_tick(&contract, number_of_ticks).bid_ask(IgnoreSize::No);

    // Test receiving data
    let subscription = result.expect("Failed to create bid/ask subscription");
    let received_ticks: Vec<BidAsk> = subscription.iter_data().take(1).map(|r| r.expect("subscription error")).collect();

    assert_eq!(received_ticks.len(), 1, "Should receive 1 bid/ask tick");

    // Verify tick data
    let tick = &received_ticks[0];
    assert_eq!(tick.bid_price, 3895.50, "Wrong bid price");
    assert_eq!(tick.ask_price, 3896.00, "Wrong ask price");
    assert_eq!(tick.bid_size, 9.0, "Wrong bid size");
    assert_eq!(tick.ask_size, 11.0, "Wrong ask size");

    assert_request(
        &message_bus,
        0,
        &tick_by_tick_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .tick_type("BidAsk")
            .number_of_ticks(number_of_ticks)
            .ignore_size(false),
    );
}

#[test]
fn test_tick_by_tick_midpoint() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::TickByTick,
            mid_point_tick().time(1678740829).mid_point(3895.375).encode_proto(),
        ),
        proto_response(
            IncomingMessages::TickByTick,
            mid_point_tick().time(1678740830).mid_point(3895.425).encode_proto(),
        ),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::TICK_BY_TICK);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let number_of_ticks = 0;

    // Test subscription creation
    let midpoints = client.tick_by_tick(&contract, number_of_ticks).mid_point();
    let midpoints = midpoints.expect("Failed to create tick-by-tick midpoint subscription");

    // Test receiving data
    let received_midpoints: Vec<MidPoint> = midpoints.iter_data().take(2).map(|r| r.expect("subscription error")).collect();

    assert_eq!(received_midpoints.len(), 2, "Should receive 2 midpoint updates");

    // Verify first midpoint
    let midpoint = &received_midpoints[0];
    assert_eq!(midpoint.mid_point, 3895.375, "Wrong midpoint for first update");
    assert_eq!(
        midpoint.time,
        OffsetDateTime::from_unix_timestamp(1678740829).unwrap(),
        "Wrong timestamp for first update"
    );

    // Verify second midpoint
    let midpoint = &received_midpoints[1];
    assert_eq!(midpoint.mid_point, 3895.425, "Wrong midpoint for second update");
    assert_eq!(
        midpoint.time,
        OffsetDateTime::from_unix_timestamp(1678740830).unwrap(),
        "Wrong timestamp for second update"
    );

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &tick_by_tick_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .tick_type("MidPoint")
            .number_of_ticks(number_of_ticks)
            .ignore_size(false),
    );
}

#[test]
fn test_basic_market_data() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        // tick_type 1 = Bid; size present + Bid → PriceSize variant.
        proto_response(
            IncomingMessages::TickPrice,
            tick_price().tick_type(1).price(185.50).size(100.0).encode_proto(),
        ),
        // tick_type 0 = BidSize
        proto_response(IncomingMessages::TickSize, tick_size().tick_type(0).size(150.0).encode_proto()),
        // tick_type 45 = LastTimestamp
        proto_response(
            IncomingMessages::TickString,
            tick_string().tick_type(45).value("2023-03-13 09:30:00").encode_proto(),
        ),
        // tick_type 23 = OptionHistoricalVol
        proto_response(IncomingMessages::TickGeneric, tick_generic().tick_type(23).value(20.5).encode_proto()),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL").build();
    let generic_ticks = &["100", "101", "104", "106"]; // Option Volume, OI, Historical Vol, Implied Vol

    // Test subscription creation
    let result = client.market_data(&contract).generic_ticks(generic_ticks).subscribe();

    // Test receiving data
    let subscription = result.expect("Failed to create market data subscription");
    let received_ticks: Vec<TickTypes> = subscription.iter_data().take(4).map(|r| r.expect("subscription error")).collect();

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
            TickTypes::PriceSize(tick) => {
                assert_eq!(tick.price_tick_type, TickType::Bid, "Wrong tick type");
                assert_eq!(tick.price, 185.50, "Wrong price");
                assert_eq!(tick.attributes.can_auto_execute, false, "Wrong can auto execute flag");
                assert_eq!(tick.size_tick_type, TickType::BidSize, "Wrong tick type");
                assert_eq!(tick.size, 100.0, "Wrong size");
            }
            TickTypes::String(tick) => {
                assert_eq!(tick.tick_type, TickType::LastTimestamp, "Wrong tick type");
                assert_eq!(tick.value, "2023-03-13 09:30:00", "Wrong timestamp");
            }
            TickTypes::Generic(tick) => {
                assert_eq!(tick.tick_type, TickType::OptionHistoricalVol, "Wrong tick type");
                assert_eq!(tick.value, 20.5, "Wrong value");
            }
            _ => panic!("Unexpected tick type received: {tick:?}"),
        }
    }

    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(generic_ticks),
    );
}

#[test]
fn test_market_data_with_combo_legs() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::PRICE_BASED_VOLATILITY);
    let mut contract = Contract::stock("AAPL").build();
    contract.security_type = SecurityType::Spread;
    contract.combo_legs = vec![ComboLeg {
        contract_id: 12345,
        ratio: 1,
        action: LegAction::Buy,
        exchange: "SMART".to_owned(),
        ..ComboLeg::default()
    }];
    let generic_ticks: Vec<&str> = vec!["233", "456"];

    // Test subscription creation
    let result = client.market_data(&contract).generic_ticks(&generic_ticks).subscribe();
    assert!(result.is_ok(), "Failed to create market data subscription with combo legs");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(&generic_ticks),
    );
}

#[test]
fn test_market_data_with_delta_neutral() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::PRICE_BASED_VOLATILITY);
    let mut contract = Contract::stock("AAPL").build();
    contract.delta_neutral_contract = Some(DeltaNeutralContract {
        contract_id: 12345,
        delta: 0.5,
        price: 100.0,
    });
    let generic_ticks: Vec<&str> = vec![];

    // Test subscription creation
    let result = client.market_data(&contract).generic_ticks(&generic_ticks).subscribe();
    assert!(result.is_ok(), "Failed to create market data subscription with delta neutral");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(&generic_ticks),
    );
}

#[test]
fn test_market_data_regulatory_snapshot() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
        ordered_responses: vec![],
    });

    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_SMART_COMPONENTS);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let generic_ticks: Vec<&str> = vec![];

    // Test subscription creation
    let result = client
        .market_data(&contract)
        .generic_ticks(&generic_ticks)
        .snapshot()
        .regulatory_snapshot()
        .subscribe();
    assert!(result.is_ok(), "Failed to create regulatory snapshot market data subscription");

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &market_data_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .generic_ticks(&generic_ticks)
            .snapshot(true)
            .regulatory_snapshot(true),
    );
}

#[test]
fn test_tick_by_tick_last() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
        IncomingMessages::TickByTick,
        trade_tick()
            .tick_type(1)
            .time(1678740829)
            .price(3895.25)
            .size(7.0)
            .attributes(false, true)
            .exchange("NASDAQ")
            .special_conditions("Regular")
            .encode_proto(),
    )]));

    let client = Client::stubbed(message_bus.clone(), server_versions::TICK_BY_TICK_IGNORE_SIZE);
    let contract = Contract {
        symbol: Symbol::from("GBL"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("EUREX"),
        currency: Currency::from("EUR"),
        last_trade_date_or_contract_month: "202303".to_owned(),
        ..Contract::default()
    };
    let number_of_ticks = 1;

    // Test subscription creation
    let result = client.tick_by_tick(&contract, number_of_ticks).last();

    // Test receiving data
    let trades = result.expect("Failed to receive tick-by-tick last data");
    let received_trades: Vec<Trade> = trades.iter_data().take(1).map(|r| r.expect("subscription error")).collect();

    assert_eq!(received_trades.len(), 1, "Should receive 1 trade");

    // Verify trade data
    let trade = &received_trades[0];
    assert_eq!(trade.price, 3895.25, "Wrong price");
    assert_eq!(trade.size, 7.0, "Wrong size");
    assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange");

    // Builder verifies tick_type "Last" (not "AllLast").
    assert_request(
        &message_bus,
        0,
        &tick_by_tick_request()
            .request_id(TEST_REQ_ID_FIRST)
            .contract(&contract)
            .tick_type("Last")
            .number_of_ticks(number_of_ticks)
            .ignore_size(false),
    );
}
