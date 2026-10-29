//! [`Quote`]: market data ticks folded into one value.

use super::TickTypes;
use crate::contracts::tick_types::TickType;
use crate::contracts::OptionComputation;
use crate::market_data::MarketDataType;

/// Latest known market data for one contract, folded from [`TickTypes`].
///
/// Build one from a snapshot with [`Quote::from_ticks`], or keep one current from a streaming
/// subscription with [`Quote::apply`]. Every field is `None` until a tick for it arrives, and
/// the latest tick wins.
///
/// Delayed ticks (`DelayedBid`, `DelayedLast`, …) fill the same fields as their real-time
/// counterparts; [`market_data_type`](Self::market_data_type) says which kind of data the quote
/// holds. Option computations are routed by their `field` into the bid/ask/last/model greeks.
/// String, generic, snapshot-end and request-parameter ticks, and tick types without a field
/// here, are ignored.
///
/// Values are stored as TWS sends them: TWS may report a missing bid or ask as `-1.0` rather
/// than omitting it.
///
/// The struct is `#[non_exhaustive]` so fields can be added; outside this crate, build one with
/// [`Quote::default`] plus [`apply`](Self::apply), or with [`from_ticks`](Self::from_ticks).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Quote {
    /// Highest bid price.
    pub bid: Option<f64>,
    /// Lowest ask price.
    pub ask: Option<f64>,
    /// Last traded price.
    pub last: Option<f64>,
    /// Session opening price.
    pub open: Option<f64>,
    /// Session high.
    pub high: Option<f64>,
    /// Session low.
    pub low: Option<f64>,
    /// Previous session's closing price.
    pub close: Option<f64>,
    /// Size at the bid.
    pub bid_size: Option<f64>,
    /// Size at the ask.
    pub ask_size: Option<f64>,
    /// Size of the last trade.
    pub last_size: Option<f64>,
    /// Session volume.
    pub volume: Option<f64>,
    /// Kind of data received (real-time, frozen, delayed, delayed-frozen).
    pub market_data_type: Option<MarketDataType>,
    /// Option model values computed from the bid.
    pub bid_greeks: Option<OptionComputation>,
    /// Option model values computed from the ask.
    pub ask_greeks: Option<OptionComputation>,
    /// Option model values computed from the last trade.
    pub last_greeks: Option<OptionComputation>,
    /// Option model values from TWS's model price.
    pub model_greeks: Option<OptionComputation>,
}

impl Quote {
    /// Folds `ticks` into a new quote, applying each in order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # {
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::realtime::Quote;
    /// use std::time::Duration;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let ticks = client.market_data(&contract).snapshot_once(Duration::from_secs(5)).expect("snapshot failed");
    /// let quote = Quote::from_ticks(&ticks);
    /// println!("bid {:?} ask {:?} last {:?}", quote.bid, quote.ask, quote.last);
    /// # }
    /// ```
    pub fn from_ticks<'a>(ticks: impl IntoIterator<Item = &'a TickTypes>) -> Self {
        let mut quote = Self::default();
        for tick in ticks {
            quote.apply(tick);
        }
        quote
    }

    /// Updates the quote with one tick. Ticks without a field here are ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::prelude::*;
    ///
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let mut ticks = client.market_data(&contract).subscribe().await?.filter_data();
    /// let mut quote = Quote::default();
    /// while let Some(tick) = ticks.next().await {
    ///     quote.apply(&tick?);
    ///     println!("bid {:?} ask {:?}", quote.bid, quote.ask);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply(&mut self, tick: &TickTypes) {
        match tick {
            TickTypes::Price(tick) => self.set_price(tick.tick_type, tick.price),
            TickTypes::Size(tick) => self.set_size(tick.tick_type, tick.size),
            TickTypes::PriceSize(tick) => {
                self.set_price(tick.price_tick_type, tick.price);
                self.set_size(tick.size_tick_type, tick.size);
            }
            TickTypes::OptionComputation(computation) => self.set_greeks(computation),
            TickTypes::MarketDataType(market_data_type) => self.market_data_type = Some(*market_data_type),
            TickTypes::String(_) | TickTypes::Generic(_) | TickTypes::SnapshotEnd | TickTypes::RequestParameters(_) => {}
        }
    }

    fn set_price(&mut self, tick_type: TickType, price: f64) {
        let slot = match tick_type {
            TickType::Bid | TickType::DelayedBid => &mut self.bid,
            TickType::Ask | TickType::DelayedAsk => &mut self.ask,
            TickType::Last | TickType::DelayedLast => &mut self.last,
            TickType::Open | TickType::DelayedOpen => &mut self.open,
            TickType::High | TickType::DelayedHigh => &mut self.high,
            TickType::Low | TickType::DelayedLow => &mut self.low,
            TickType::Close | TickType::DelayedClose => &mut self.close,
            _ => return,
        };
        *slot = Some(price);
    }

    fn set_size(&mut self, tick_type: TickType, size: f64) {
        let slot = match tick_type {
            TickType::BidSize | TickType::DelayedBidSize => &mut self.bid_size,
            TickType::AskSize | TickType::DelayedAskSize => &mut self.ask_size,
            TickType::LastSize | TickType::DelayedLastSize => &mut self.last_size,
            TickType::Volume | TickType::DelayedVolume => &mut self.volume,
            _ => return,
        };
        *slot = Some(size);
    }

    fn set_greeks(&mut self, computation: &OptionComputation) {
        let slot = match computation.field {
            TickType::BidOption | TickType::DelayedBidOption => &mut self.bid_greeks,
            TickType::AskOption | TickType::DelayedAskOption => &mut self.ask_greeks,
            TickType::LastOption | TickType::DelayedLastOption => &mut self.last_greeks,
            TickType::ModelOption | TickType::DelayedModelOption => &mut self.model_greeks,
            _ => return,
        };
        *slot = Some(computation.clone());
    }
}

#[cfg(test)]
#[path = "quote_tests.rs"]
mod tests;
