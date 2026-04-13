use log::debug;

use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::protocol::{check_version, Features};
use crate::{client::sync::Client, server_versions, Error};

use super::common::{decoders, encoders};
use super::{Bar, BarSize, BidAsk, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, Trade, WhatToShow};
use crate::market_data::TradingHours;

// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    check_version(client.server_version(), Features::TICK_BY_TICK)?;

    if number_of_ticks != 0 || ignore_size {
        check_version(client.server_version(), Features::TICK_BY_TICK_IGNORE_SIZE)?;
    }

    Ok(())
}

impl Client {
    /// Requests realtime bars.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::{BarSize, WhatToShow};
    /// use ibapi::market_data::TradingHours;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    /// let subscription = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended).expect("request failed");
    ///
    /// for (i, bar) in subscription.iter().enumerate().take(60) {
    ///     println!("bar[{i}]: {bar:?}");
    /// }
    /// ```
    pub fn realtime_bars(
        &self,
        contract: &Contract,
        _bar_size: BarSize,
        what_to_show: WhatToShow,
        trading_hours: TradingHours,
    ) -> Result<Subscription<Bar>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_realtime_bars(
            builder.request_id(),
            contract,
            &what_to_show,
            trading_hours.use_rth(),
            &Vec::<TagValue>::default(),
        )?;

        builder.send(request)
    }

    /// Requests tick by tick AllLast ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_all_last(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick all last data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("All Last Tick: {tick:?}");
    /// }
    /// ```
    pub fn tick_by_tick_all_last(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "AllLast", number_of_ticks, ignore_size)?;

        builder.send(request)
    }

    /// Requests tick by tick BidAsk ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored. (typically true for BidAsk ticks to get changes based on price).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick bid/ask data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("BidAsk Tick: {tick:?}");
    /// }
    /// ```
    pub fn tick_by_tick_bid_ask(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<BidAsk>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "BidAsk", number_of_ticks, ignore_size)?;

        builder.send(request)
    }

    /// Requests tick by tick Last ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored (typically false for Last ticks).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_last(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick last data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("Last Tick: {tick:?}");
    /// }
    /// ```
    pub fn tick_by_tick_last(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "Last", number_of_ticks, ignore_size)?;

        builder.send(request)
    }

    /// Requests tick by tick MidPoint ticks.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] for which to request tick-by-tick data.
    /// * `number_of_ticks` - The number of ticks to retrieve. TWS usually limits this to 1000.
    /// * `ignore_size`     - Specifies if tick sizes should be ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let number_of_ticks = 10; // Request a small number of ticks for the example
    /// let ignore_size = false;
    ///
    /// let subscription = client.tick_by_tick_bid_ask(&contract, number_of_ticks, ignore_size)
    ///     .expect("tick-by-tick mid-point data request failed");
    ///
    /// for tick in subscription.iter().take(number_of_ticks as usize) { // Take to limit example output
    ///     println!("MidPoint Tick: {tick:?}");
    /// }
    /// ```
    pub fn tick_by_tick_midpoint(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<MidPoint>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "MidPoint", number_of_ticks, ignore_size)?;

        builder.send(request)
    }

    /// Requests the contract's market depth (order book).
    ///
    /// # Arguments
    ///
    /// * `contract` - The Contract for which the depth is being requested.
    /// * `number_of_rows` - The number of rows on each side of the order book.
    /// * `is_smart_depth` - Flag indicates that this is smart depth request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let subscription = client.market_depth(&contract, 5, true).expect("error requesting market depth");
    /// for row in &subscription {
    ///     println!("row: {row:?}");
    /// }
    ///
    /// if let Some(error) = subscription.error() {
    ///     println!("error: {error:?}");
    /// }
    /// ```
    pub fn market_depth(&self, contract: &Contract, number_of_rows: i32, is_smart_depth: bool) -> Result<Subscription<MarketDepths>, Error> {
        if is_smart_depth {
            check_version(self.server_version(), Features::SMART_DEPTH)?;
        }
        if !contract.primary_exchange.is_empty() {
            check_version(self.server_version(), Features::MKT_DEPTH_PRIM_EXCHANGE)?;
        }

        let builder = self.request();
        let request = encoders::encode_request_market_depth(builder.request_id(), contract, number_of_rows, is_smart_depth)?;

        builder.send_with_context(request, self.decoder_context().with_smart_depth(is_smart_depth))
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
                Some(Ok(mut message)) => return decoders::decode_market_depth_exchanges(self.server_version(), &mut message),
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

#[cfg(test)]
mod tests;
