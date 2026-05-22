use log::debug;

use crate::client::ClientRequestBuilders;
use crate::contracts::{Contract, TagValue};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

use super::common::{decoders, encoders};
use super::{Bar, DepthMarketDataDescription, MarketDepths, RealtimeBarsBuilder, TickByTickBuilder, TickTypes, WhatToShow};
use crate::market_data::TradingHours;
use crate::subscriptions::StreamDecoder;

impl Client {
    /// Switches market data type returned from market data request.
    pub async fn switch_market_data_type(&self, market_data_type: crate::market_data::MarketDataType) -> Result<(), Error> {
        self.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

        let message = crate::market_data::encoders::encode_request_market_data_type(market_data_type)?;
        self.send_message(message).await
    }

    /// Returns a builder for a real-time 5-second bar subscription.
    ///
    /// Defaults to `WhatToShow::Trades` and `TradingHours::Regular`. See
    /// [`RealtimeBarsBuilder`] for the chained methods.
    pub fn realtime_bars<'a>(&'a self, contract: &'a Contract) -> RealtimeBarsBuilder<'a, Self> {
        RealtimeBarsBuilder::new(self, contract)
    }

    pub(crate) async fn subscribe_realtime_bars(
        &self,
        contract: &Contract,
        what_to_show: &WhatToShow,
        trading_hours: TradingHours,
        options: &[TagValue],
    ) -> Result<Subscription<Bar>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_realtime_bars(builder.request_id(), contract, what_to_show, trading_hours.use_rth(), options)?;

        builder.send::<Bar>(request).await
    }

    /// Returns a builder for a tick-by-tick real-time subscription.
    ///
    /// Pick the tick stream with the terminal — `.last()` / `.all_last()` /
    /// `.bid_ask(IgnoreSize)` / `.mid_point()`. See [`TickByTickBuilder`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let mut quotes = client
    ///         .tick_by_tick(&contract, 10)
    ///         .bid_ask(IgnoreSize::No)
    ///         .await
    ///         .expect("tick-by-tick bid/ask request failed");
    ///
    ///     while let Some(item) = quotes.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(q)) => println!("{q:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => { eprintln!("error: {e}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn tick_by_tick<'a>(&'a self, contract: &'a Contract, number_of_ticks: i32) -> TickByTickBuilder<'a, Self> {
        TickByTickBuilder::new(self, contract, number_of_ticks)
    }

    /// Requests market depth data.
    pub async fn market_depth(&self, contract: &Contract, number_of_rows: i32, is_smart_depth: bool) -> Result<Subscription<MarketDepths>, Error> {
        if is_smart_depth {
            check_version(self.server_version(), Features::SMART_DEPTH)?;
        }
        if !contract.primary_exchange.is_empty() {
            check_version(self.server_version(), Features::MKT_DEPTH_PRIM_EXCHANGE)?;
        }

        let builder = self.request();
        let request = encoders::encode_request_market_depth(builder.request_id(), contract, number_of_rows, is_smart_depth)?;

        builder
            .send_with_context::<MarketDepths>(request, self.decoder_context().with_smart_depth(is_smart_depth))
            .await
    }

    /// Requests venues for which market data is returned to market_depth (those with market makers)
    pub async fn market_depth_exchanges(&self) -> Result<Vec<DepthMarketDataDescription>, Error> {
        check_version(self.server_version(), Features::REQ_MKT_DEPTH_EXCHANGES)?;

        loop {
            let request = encoders::encode_request_market_depth_exchanges()?;
            let mut subscription = self.shared_request(OutgoingMessages::RequestMktDepthExchanges).send_raw(request).await?;
            let response = subscription.next().await;

            match response {
                Some(Ok(mut message)) => return decoders::decode_market_depth_exchanges(self.server_version(), &mut message),
                Some(Err(e)) => return Err(e),
                None => {
                    debug!("connection reset. retrying market_depth_exchanges");
                    continue;
                }
            }
        }
    }

    /// Requests real time market data (low-level).
    pub(crate) async fn subscribe_market_data(
        &self,
        contract: &Contract,
        generic_ticks: &[&str],
        snapshot: bool,
        regulatory_snapshot: bool,
    ) -> Result<Subscription<TickTypes>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_market_data(builder.request_id(), contract, generic_ticks, snapshot, regulatory_snapshot)?;

        builder.send::<TickTypes>(request).await
    }
}

/// Validates that server supports the given request.
pub(super) fn validate_tick_by_tick_request(client: &Client, _contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<(), Error> {
    check_version(client.server_version(), Features::TICK_BY_TICK)?;

    if number_of_ticks != 0 || ignore_size {
        check_version(client.server_version(), Features::TICK_BY_TICK_IGNORE_SIZE)?;
    }

    Ok(())
}

pub(crate) async fn tick_by_tick<T: StreamDecoder<T> + Send + 'static>(
    client: &Client,
    contract: &Contract,
    tick_type: &str,
    number_of_ticks: i32,
    ignore_size: bool,
) -> Result<Subscription<T>, Error> {
    validate_tick_by_tick_request(client, contract, number_of_ticks, ignore_size)?;

    let builder = client.request();
    let request = encoders::encode_tick_by_tick(builder.request_id(), contract, tick_type, number_of_ticks, ignore_size)?;
    builder.send::<T>(request).await
}

#[cfg(test)]
mod tests;
