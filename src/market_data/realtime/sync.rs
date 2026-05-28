use log::debug;

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::contracts::{Contract, TagValue};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::{client::sync::Client, server_versions, Error};

use super::common::{decoders, encoders};
use super::{Bar, DepthMarketDataDescription, MarketDepthBuilder, MarketDepths, RealtimeBarsBuilder, TickByTickBuilder, TickTypes, WhatToShow};
use crate::market_data::{SmartDepth, TradingHours};
use crate::subscriptions::StreamDecoder;

// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    check_version(client.server_version(), Features::TICK_BY_TICK)?;

    if number_of_ticks != 0 || ignore_size {
        check_version(client.server_version(), Features::TICK_BY_TICK_IGNORE_SIZE)?;
    }

    Ok(())
}

impl Client {
    /// Returns a builder for a real-time 5-second bar subscription.
    ///
    /// Defaults to `WhatToShow::Trades` and `TradingHours::Regular`. See
    /// [`RealtimeBarsBuilder`] for the chained methods.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::WhatToShow;
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    /// let subscription = client
    ///     .realtime_bars(&contract)
    ///     .what_to_show(WhatToShow::Trades)
    ///     .trading_hours(TradingHours::Extended)
    ///     .subscribe()
    ///     .expect("realtime bars request failed");
    ///
    /// for (i, bar) in subscription.iter_data().enumerate().take(60) {
    ///     match bar {
    ///         Ok(bar) => println!("bar[{i}]: {bar:?}"),
    ///         Err(e) => { eprintln!("error: {e:?}"); break; }
    ///     }
    /// }
    /// ```
    pub fn realtime_bars<'a>(&'a self, contract: &'a Contract) -> RealtimeBarsBuilder<'a, Self> {
        RealtimeBarsBuilder::new(self, contract)
    }

    /// Returns a builder for a tick-by-tick real-time subscription.
    ///
    /// Pick the tick stream with the terminal — `.last()` / `.all_last()` /
    /// `.bid_ask(IgnoreSize)` / `.mid_point()`. See [`TickByTickBuilder`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::IgnoreSize;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let quotes = client
    ///     .tick_by_tick(&contract, 10)
    ///     .bid_ask(IgnoreSize::No)
    ///     .expect("tick-by-tick bid/ask request failed");
    ///
    /// for quote in quotes.iter_data().take(10) {
    ///     match quote {
    ///         Ok(quote) => println!("{quote:?}"),
    ///         Err(e) => { eprintln!("error: {e:?}"); break; }
    ///     }
    /// }
    /// ```
    pub fn tick_by_tick<'a>(&'a self, contract: &'a Contract, number_of_ticks: i32) -> TickByTickBuilder<'a, Self> {
        TickByTickBuilder::new(self, contract, number_of_ticks)
    }

    /// Returns a builder for a level-2 market-depth (order book) subscription.
    ///
    /// Defaults to `SmartDepth::No`. See [`MarketDepthBuilder`] for the chained
    /// methods.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::SmartDepth;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let subscription = client
    ///     .market_depth(&contract, 5)
    ///     .smart_depth(SmartDepth::Yes)
    ///     .subscribe()
    ///     .expect("error requesting market depth");
    ///
    /// for row in subscription.iter_data() {
    ///     match row {
    ///         Ok(row) => println!("row: {row:?}"),
    ///         Err(e) => {
    ///             eprintln!("error: {e:?}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn market_depth<'a>(&'a self, contract: &'a Contract, number_of_rows: i32) -> MarketDepthBuilder<'a, Self> {
        MarketDepthBuilder::new(self, contract, number_of_rows)
    }

    /// Requests venues for which market data is returned to market_depth (those with market makers)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let exchanges = client.market_depth_exchanges().expect("error requesting market depth exchanges");
    /// for exchange in &exchanges {
    ///     println!("{exchange:?}");
    /// }
    /// ```
    pub fn market_depth_exchanges(&self) -> Result<Vec<DepthMarketDataDescription>, Error> {
        check_version(self.server_version(), Features::REQ_MKT_DEPTH_EXCHANGES)?;

        loop {
            let request = encoders::encode_request_market_depth_exchanges()?;
            let subscription = self.shared_request(OutgoingMessages::RequestMktDepthExchanges).send_raw(request)?;
            let response = subscription.next();

            match response {
                Some(Ok(message)) => return decoders::decode_market_depth_exchanges(&message),
                Some(Err(Error::ConnectionReset)) => {
                    debug!("connection reset. retrying market_depth_exchanges");
                    continue;
                }
                Some(Err(e)) => return Err(e),
                None => return Ok(Vec::new()),
            }
        }
    }

    /// Switches market data type returned from request_market_data requests to Live, Frozen, Delayed, or FrozenDelayed.
    ///
    /// # Arguments
    /// * `market_data_type` - Type of market data to retrieve.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::market_data::{MarketDataType};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let market_data_type = MarketDataType::Realtime;
    /// client.switch_market_data_type(market_data_type).expect("request failed");
    /// println!("market data switched: {market_data_type:?}");
    /// ```
    pub fn switch_market_data_type(&self, market_data_type: crate::market_data::MarketDataType) -> Result<(), Error> {
        self.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

        let message = crate::market_data::encoders::encode_request_market_data_type(market_data_type)?;
        let _ = self.send_shared_request(OutgoingMessages::RequestMarketDataType, message)?;
        Ok(())
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
    let request = encoders::encode_request_market_data(builder.request_id(), contract, generic_ticks, snapshot, regulatory_snapshot)?;

    builder.send(request)
}

pub(crate) fn realtime_bars(
    client: &Client,
    contract: &Contract,
    what_to_show: &WhatToShow,
    trading_hours: TradingHours,
    options: &[TagValue],
) -> Result<Subscription<Bar>, Error> {
    let builder = client.request();
    let request = encoders::encode_request_realtime_bars(builder.request_id(), contract, what_to_show, trading_hours.use_rth(), options)?;

    builder.send(request)
}

pub(crate) fn market_depth(
    client: &Client,
    contract: &Contract,
    number_of_rows: i32,
    smart_depth: SmartDepth,
) -> Result<Subscription<MarketDepths>, Error> {
    let is_smart_depth = matches!(smart_depth, SmartDepth::Yes);
    if is_smart_depth {
        check_version(client.server_version(), Features::SMART_DEPTH)?;
    }
    if !contract.primary_exchange.is_empty() {
        check_version(client.server_version(), Features::MKT_DEPTH_PRIM_EXCHANGE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_market_depth(builder.request_id(), contract, number_of_rows, is_smart_depth)?;
    builder.send_with_context(request, client.decoder_context().with_smart_depth(is_smart_depth))
}

pub(crate) fn tick_by_tick<T: StreamDecoder<T>>(
    client: &Client,
    contract: &Contract,
    tick_type: &str,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<T>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let builder = client.request();
    let request = encoders::encode_tick_by_tick(builder.request_id(), contract, tick_type, number_of_ticks, ignore_size)?;
    builder.send(request)
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
