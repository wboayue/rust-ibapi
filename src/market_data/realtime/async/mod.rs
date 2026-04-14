use log::debug;

use crate::client::ClientRequestBuilders;
use crate::contracts::{Contract, TagValue};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

use super::common::{decoders, encoders};
use super::{Bar, BarSize, BidAsk, DepthMarketDataDescription, MarketDepths, MidPoint, TickTypes, Trade, WhatToShow};
use crate::market_data::TradingHours;

impl Client {
    /// Switches market data type returned from market data request.
    pub async fn switch_market_data_type(&self, market_data_type: crate::market_data::MarketDataType) -> Result<(), Error> {
        self.check_server_version(server_versions::REQ_MARKET_DATA_TYPE, "It does not support market data type requests.")?;

        let message = crate::market_data::encoders::encode_request_market_data_type(market_data_type)?;
        self.send_message(message).await
    }

    /// Requests realtime bars.
    pub async fn realtime_bars(
        &self,
        contract: &Contract,
        _bar_size: &BarSize,
        what_to_show: &WhatToShow,
        trading_hours: TradingHours,
        options: Vec<TagValue>,
    ) -> Result<Subscription<Bar>, Error> {
        let builder = self.request();
        let request = encoders::encode_request_realtime_bars(builder.request_id(), contract, what_to_show, trading_hours.use_rth(), &options)?;

        builder.send::<Bar>(request).await
    }

    /// Requests tick by tick AllLast ticks.
    pub async fn tick_by_tick_all_last(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "AllLast", number_of_ticks, ignore_size)?;

        builder.send::<Trade>(request).await
    }

    /// Requests tick by tick Last ticks.
    pub async fn tick_by_tick_last(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<Trade>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "Last", number_of_ticks, ignore_size)?;

        builder.send::<Trade>(request).await
    }

    /// Requests tick by tick BidAsk ticks.
    pub async fn tick_by_tick_bid_ask(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<BidAsk>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "BidAsk", number_of_ticks, ignore_size)?;

        builder.send::<BidAsk>(request).await
    }

    /// Requests tick by tick MidPoint ticks.
    pub async fn tick_by_tick_midpoint(&self, contract: &Contract, number_of_ticks: i32, ignore_size: bool) -> Result<Subscription<MidPoint>, Error> {
        validate_tick_by_tick_request(self, contract, number_of_ticks, ignore_size)?;

        let builder = self.request();

        let request = encoders::encode_tick_by_tick(builder.request_id(), contract, "MidPoint", number_of_ticks, ignore_size)?;

        builder.send::<MidPoint>(request).await
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

#[cfg(test)]
mod tests;
