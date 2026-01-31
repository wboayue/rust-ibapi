use log::debug;

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::protocol::{check_version, Features};
use crate::{client::sync::Client, Error};

use super::common::{decoders, encoders};
use super::{Bar, BarSize, BidAsk, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, Trade, WhatToShow};
use crate::market_data::TradingHours;

// Requests realtime bars.
pub(crate) fn realtime_bars(
    client: &Client,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    trading_hours: TradingHours,
    options: Vec<TagValue>,
) -> Result<Subscription<Bar>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_realtime_bars(
        client.server_version(),
        builder.request_id(),
        contract,
        bar_size,
        what_to_show,
        trading_hours.use_rth(),
        options,
    )?;

    builder.send(request)
}

// Requests tick by tick AllLast ticks.
pub(crate) fn tick_by_tick_all_last(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "AllLast", number_of_ticks, ignore_size)?;

    builder.send(request)
}

// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    check_version(client.server_version(), Features::TICK_BY_TICK)?;

    if number_of_ticks != 0 || ignore_size {
        check_version(client.server_version(), Features::TICK_BY_TICK_IGNORE_SIZE)?;
    }

    Ok(())
}

// Requests tick by tick Last ticks.
pub(crate) fn tick_by_tick_last(client: &Client, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "Last", number_of_ticks, ignore_size)?;

    builder.send(request)
}

// Requests tick by tick BidAsk ticks.
pub(crate) fn tick_by_tick_bid_ask(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<BidAsk>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "BidAsk", number_of_ticks, ignore_size)?;

    builder.send(request)
}

// Requests tick by tick MidPoint ticks.
pub(crate) fn tick_by_tick_midpoint(
    client: &Client,
    contract: &Contract,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<MidPoint>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let server_version = client.server_version();
    let builder = client.request();

    let request = encoders::encode_tick_by_tick(server_version, builder.request_id(), contract, "MidPoint", number_of_ticks, ignore_size)?;

    builder.send(request)
}

pub(crate) fn market_depth(
    client: &Client,
    contract: &Contract,
    number_of_rows: i32,
    is_smart_depth: bool,
) -> Result<Subscription<MarketDepths>, Error> {
    if is_smart_depth {
        check_version(client.server_version(), Features::SMART_DEPTH)?;
    }
    if !contract.primary_exchange.is_empty() {
        check_version(client.server_version(), Features::MKT_DEPTH_PRIM_EXCHANGE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_market_depth(client.server_version(), builder.request_id(), contract, number_of_rows, is_smart_depth)?;

    builder.send_with_context(request, client.decoder_context().with_smart_depth(is_smart_depth))
}

/// Fetch the venues that provide market depth data for the connected account.
pub fn market_depth_exchanges(client: &Client) -> Result<Vec<DepthMarketDataDescription>, Error> {
    check_version(client.server_version(), Features::REQ_MKT_DEPTH_EXCHANGES)?;

    loop {
        let request = encoders::encode_request_market_depth_exchanges()?;
        let subscription = client.shared_request(OutgoingMessages::RequestMktDepthExchanges).send_raw(request)?;
        let response = subscription.next();

        match response {
            Some(Ok(mut message)) => return decoders::decode_market_depth_exchanges(client.server_version(), &mut message),
            Some(Err(Error::ConnectionReset)) => {
                debug!("connection reset. retrying market_depth_exchanges");
                continue;
            }
            Some(Err(e)) => return Err(e),
            None => return Ok(Vec::new()),
        }
    }
}

/// Subscribe to streaming level-1 market data for the given contract.
pub fn market_data(
    client: &Client,
    contract: &Contract,
    generic_ticks: &[&str],
    snapshot: bool,
    regulatory_snapshot: bool,
) -> Result<Subscription<TickTypes>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_market_data(
        client.server_version(),
        builder.request_id(),
        contract,
        generic_ticks,
        snapshot,
        regulatory_snapshot,
    )?;

    builder.send(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::tick_types::TickType;
    use crate::contracts::{ComboLeg, Contract, Currency, DeltaNeutralContract, Exchange, SecurityType, Symbol};
    use crate::market_data::TradingHours;
    use crate::messages::OutgoingMessages;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::ToField;
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
        // Setup test message bus with mock responses
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "50|3|9001|1678323335|4028.75|4029.00|4028.25|4028.50|2|4026.75|1|".to_owned(),
                "50|3|9001|1678323340|4028.80|4029.10|4028.30|4028.55|3|4026.80|2|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract {
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            local_symbol: "FGBL MAR 23".to_owned(),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let bar_size = BarSize::Sec5;
        let what_to_show = WhatToShow::Trades;
        let trading_hours = TradingHours::Regular;

        // Test subscription creation
        let bars = client.realtime_bars(&contract, bar_size, what_to_show, trading_hours);

        // Test receiving data
        let bars = bars.expect("Failed to create realtime bars subscription");
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
        assert_eq!(request[17], trading_hours.use_rth().to_field(), "Wrong use RTH flag");
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

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let number_of_ticks = 2;
        let ignore_size = false;

        // Test subscription creation
        let trades = client.tick_by_tick_all_last(&contract, number_of_ticks, ignore_size);
        let trades = trades.expect("Failed to create tick-by-tick subscription");

        // Test receiving data
        let received_trades: Vec<Trade> = trades.iter().take(2).collect();

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

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestTickByTickData.to_field(), "Wrong message type");
        assert_eq!(request[14], "AllLast", "Wrong tick type");
    }

    #[test]
    fn test_market_depth() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["12|1|9001|0|0|0|4028.75|100|".to_owned(), "12|1|9001|1|1|1|4028.50|200|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SMART_DEPTH);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let number_of_rows = 10;
        let is_smart_depth = false;

        // Test subscription creation
        let depth = client.market_depth(&contract, number_of_rows, is_smart_depth);
        let depth = depth.expect("Failed to create market depth subscription");

        // Test receiving data
        let received_depth: Vec<MarketDepths> = depth.iter().take(2).collect();

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

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestMarketDepth.to_field(), "Wrong message type");
        assert_eq!(request[1], "5", "Wrong version");
        assert_eq!(request[14], number_of_rows.to_field(), "Wrong number of rows");
        assert_eq!(request[15], is_smart_depth.to_field(), "Wrong smart depth flag");
    }

    #[test]
    fn test_market_depth_exchanges() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["71|2|ISLAND|STK|NASDAQ|DEEP2|1|NYSE|STK|NYSE|DEEP|1|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SERVICE_DATA_TYPE);

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

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestMktDepthExchanges.to_field(), "Wrong message type");
    }

    #[test]
    fn test_tick_by_tick_bid_ask() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["99|9001|3|1678745793|3895.50|3896.00|9|11|3|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let number_of_ticks = 1;
        let ignore_size = false;

        // Test subscription creation
        let result = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size);

        // Test receiving data
        let subscription = result.expect("Failed to create bid/ask subscription");
        let received_ticks: Vec<BidAsk> = subscription.iter().take(1).collect();

        assert_eq!(received_ticks.len(), 1, "Should receive 1 bid/ask tick");

        // Verify tick data
        let tick = &received_ticks[0];
        assert_eq!(tick.bid_price, 3895.50, "Wrong bid price");
        assert_eq!(tick.ask_price, 3896.00, "Wrong ask price");
        assert_eq!(tick.bid_size, 9.0, "Wrong bid size");
        assert_eq!(tick.ask_size, 11.0, "Wrong ask size");

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];
        assert_eq!(request[14], "BidAsk", "Wrong tick type");
    }

    #[test]
    fn test_tick_by_tick_midpoint() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["99|9001|4|1678740829|3895.375|".to_owned(), "99|9001|4|1678740830|3895.425|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let number_of_ticks = 0;
        let ignore_size = false;

        // Test subscription creation
        let midpoints = client.tick_by_tick_midpoint(&contract, number_of_ticks, ignore_size);
        let midpoints = midpoints.expect("Failed to create tick-by-tick midpoint subscription");

        // Test receiving data
        let received_midpoints: Vec<MidPoint> = midpoints.iter().take(2).collect();

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

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestTickByTickData.to_field(), "Wrong message type");
        assert_eq!(request[14], "MidPoint", "Wrong tick type");
    }

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
                "46|2|9001|45|2023-03-13 09:30:00|".to_owned(),
                // Tick Generic message
                "45|2|9001|23|20.5|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
        let contract = Contract::stock("AAPL").build();
        let generic_ticks = &["100", "101", "104", "106"]; // Option Volume, OI, Historical Vol, Implied Vol

        // Test subscription creation
        let result = client.market_data(&contract).generic_ticks(generic_ticks).subscribe();

        // Test receiving data
        let subscription = result.expect("Failed to create market data subscription");
        let received_ticks: Vec<TickTypes> = subscription.iter().take(4).collect();

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
    }

    #[test]
    fn test_market_data_with_combo_legs() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::PRICE_BASED_VOLATILITY);
        let mut contract = Contract::stock("AAPL").build();
        contract.security_type = SecurityType::Spread;
        contract.combo_legs = vec![ComboLeg {
            contract_id: 12345,
            ratio: 1,
            action: "BUY".to_owned(),
            exchange: "SMART".to_owned(),
            ..ComboLeg::default()
        }];
        let generic_ticks: Vec<&str> = vec!["233", "456"];

        // Test subscription creation
        let result = client.market_data(&contract).generic_ticks(&generic_ticks).subscribe();
        assert!(result.is_ok(), "Failed to create market data subscription with combo legs");

        // Verify request message was sent
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestMarketData.to_field(), "Wrong message type");
    }

    #[test]
    fn test_market_data_with_delta_neutral() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::PRICE_BASED_VOLATILITY);
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

        // Verify request message was sent
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestMarketData.to_field(), "Wrong message type");
    }

    #[test]
    fn test_market_data_regulatory_snapshot() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::REQ_SMART_COMPONENTS);
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

        // Verify request message
        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages.len(), 1, "Should send one request message");

        let request = &request_messages[0];
        assert_eq!(request[0], OutgoingMessages::RequestMarketData.to_field(), "Wrong message type");
        assert_eq!(request[17], "1", "Wrong regulatory snapshot flag");
    }

    #[test]
    fn test_market_data_error_handling() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                format!("4|2|9001|2104|Market data farm connection is OK:usfarm|"), // Notice
                format!("4|2|9001|321|Error validating request:-'bW' : cause - What to show field is missing or incorrect.|"), // Error
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::PRICE_BASED_VOLATILITY);
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
        let market_data = client.market_data(&contract).generic_ticks(&generic_ticks).subscribe();
        let market_data = market_data.expect("Failed to create market data subscription");

        // Test receiving data
        let mut iter = market_data.iter();

        // First should be a Notice
        match iter.next() {
            Some(TickTypes::Notice(notice)) => {
                assert_eq!(notice.code, 2104, "Wrong notice code");
                assert!(notice.message.contains("Market data farm connection is OK"), "Wrong notice message");
            }
            other => panic!("Expected Notice, got {other:?}"),
        }

        // Second should be a Notice (since it's an error in the 2100-2200 range)
        match iter.next() {
            Some(TickTypes::Notice(notice)) => {
                assert_eq!(notice.code, 321, "Wrong error code");
                assert!(notice.message.contains("Error validating request"), "Wrong error message");
            }
            other => panic!("Expected Notice for error, got {other:?}"),
        }
    }

    #[test]
    fn test_tick_by_tick_last() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["99|9001|1|1678740829|3895.25|7|2|NASDAQ|Regular|".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::TICK_BY_TICK_IGNORE_SIZE);
        let contract = Contract {
            symbol: Symbol::from("GBL"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let number_of_ticks = 1;
        let ignore_size = false;

        // Test subscription creation
        let result = client.tick_by_tick_last(&contract, number_of_ticks, ignore_size);

        // Test receiving data
        let trades = result.expect("Failed to receive tick-by-tick last data");
        let received_trades: Vec<Trade> = trades.iter().take(1).collect();

        assert_eq!(received_trades.len(), 1, "Should receive 1 trade");

        // Verify trade data
        let trade = &received_trades[0];
        assert_eq!(trade.price, 3895.25, "Wrong price");
        assert_eq!(trade.size, 7.0, "Wrong size");
        assert_eq!(trade.exchange, "NASDAQ", "Wrong exchange");

        // Verify request message uses "Last" instead of "AllLast"
        let request_messages = client.message_bus.request_messages();
        let request = &request_messages[0];
        assert_eq!(request[14], "Last", "Wrong tick type");
    }
}
