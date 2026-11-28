//! [`Quote`]: market data ticks folded into one value.

use super::{TickAttribute, TickTypes};
use crate::contracts::tick_types::TickType;
use crate::contracts::OptionComputation;
use crate::market_data::{MarketDataType, QuoteSide};

/// Latest known market data for one contract, folded from [`TickTypes`].
///
/// Build one from a snapshot with [`Quote::from_ticks`], or keep one current from a streaming
/// subscription with [`Quote::apply`]. Every field is `None` until a tick for it arrives, and
/// the latest tick wins.
///
/// Delayed ticks (`DelayedBid`, `DelayedLast`, …) fill the same fields as their real-time
/// counterparts; [`market_data_type`](Self::market_data_type) says which kind of data the quote
/// holds. Option computations are routed by their `field` into the bid/ask/last/model greeks.
/// String, generic, snapshot-end, request-parameter and reroute ticks, and tick types without a
/// field here, are ignored.
///
/// A bid or ask price tick that reports no quote, with size `0` and a placeholder price (`-1`,
/// `-1` times the price magnifier, or `0`), sets that side's price and size to `None`. Those
/// placeholders were seen on delayed and off-hours data; a price above `0` with size `0` was
/// never seen and is stored as sent. Other values are stored as TWS sends them.
/// The [`TickPrice`](super::TickPrice) and [`TickPriceSize`](super::TickPriceSize) ticks keep
/// the placeholder. [`bid_side`](Self::bid_side) and [`ask_side`](Self::ask_side) return a side
/// only when it has both a price and a positive size, so they also cover a later size tick of `0`.
///
/// The struct is `#[non_exhaustive]` so fields can be added; outside this crate, build one with
/// [`Quote::default`] plus [`apply`](Self::apply), or with [`from_ticks`](Self::from_ticks).
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Quote {
    /// Highest bid price. `None` while TWS reports no bid.
    pub bid: Option<f64>,
    /// Lowest ask price. `None` while TWS reports no ask.
    pub ask: Option<f64>,
    /// Last price reported by TWS.
    ///
    /// With [`last_size`](Self::last_size) set to `Some(0.0)`, this can be a placeholder such as
    /// `Some(-1.0)` or `Some(-100.0)` with a price magnifier of 100. EUR.USD can also report a
    /// real positive last price with size `0`, so size alone cannot identify a placeholder.
    pub last: Option<f64>,
    /// Session opening price.
    pub open: Option<f64>,
    /// Session high.
    pub high: Option<f64>,
    /// Session low.
    pub low: Option<f64>,
    /// Previous session's closing price.
    pub close: Option<f64>,
    /// Size at the bid. `None` while TWS reports no bid.
    pub bid_size: Option<f64>,
    /// Size at the ask. `None` while TWS reports no ask.
    pub ask_size: Option<f64>,
    /// Attributes of the latest bid price tick: whether it can auto-execute, is past the limit, or
    /// is a pre-open price.
    ///
    /// Delayed ticks carry no attributes: every flag is `false`
    /// (`docs/wire-facts/market-data.md#no-quote-sides`).
    pub bid_attributes: Option<TickAttribute>,
    /// Attributes of the latest ask price tick, as for [`bid_attributes`](Self::bid_attributes).
    pub ask_attributes: Option<TickAttribute>,
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
            TickTypes::Price(tick) => self.set_price(tick.tick_type, tick.price, tick.attributes),
            TickTypes::Size(tick) => self.set_size(tick.tick_type, tick.size),
            TickTypes::PriceSize(tick) => {
                self.set_price(tick.price_tick_type, tick.price, tick.attributes);
                self.set_size(tick.size_tick_type, tick.size);
                if reports_no_quote(tick.price, tick.size) {
                    self.clear_side(tick.price_tick_type);
                }
            }
            TickTypes::OptionComputation(computation) => self.set_greeks(computation),
            TickTypes::MarketDataType(market_data_type) => self.market_data_type = Some(*market_data_type),
            TickTypes::String(_) | TickTypes::Generic(_) | TickTypes::SnapshotEnd | TickTypes::RequestParameters(_) | TickTypes::Reroute(_) => {}
        }
    }

    /// The bid with its size, or `None` while the quote has no bid.
    ///
    /// TWS reports an empty side with size `0` and a placeholder price (`-1`, `-1` times the
    /// price magnifier, or `0`); see [`QuoteSide::new`]. Such a tick sets [`bid`](Self::bid)
    /// and [`bid_size`](Self::bid_size) to `None`. This is also `None` whenever the bid size is
    /// `0`, such as after a later size tick of `0` while the bid price stays.
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
    /// match Quote::from_ticks(&ticks).bid_side() {
    ///     Some(bid) => println!("bid {} x {}", bid.price, bid.size),
    ///     None => println!("no bid"),
    /// }
    /// # }
    /// ```
    pub fn bid_side(&self) -> Option<QuoteSide> {
        QuoteSide::new(self.bid?, self.bid_size?)
    }

    /// The ask with its size, or `None` while the quote has no ask. See [`bid_side`](Self::bid_side).
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
    ///     if let (Some(bid), Some(ask)) = (quote.bid_side(), quote.ask_side()) {
    ///         println!("{} x {} / {} x {}", bid.price, bid.size, ask.price, ask.size);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn ask_side(&self) -> Option<QuoteSide> {
        QuoteSide::new(self.ask?, self.ask_size?)
    }

    fn set_price(&mut self, tick_type: TickType, price: f64, attributes: TickAttribute) {
        match tick_type {
            TickType::Bid | TickType::DelayedBid => self.bid_attributes = Some(attributes),
            TickType::Ask | TickType::DelayedAsk => self.ask_attributes = Some(attributes),
            _ => {}
        }
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

    /// Clear the side a bid or ask price tick that reports no quote names.
    fn clear_side(&mut self, price_tick_type: TickType) {
        let (price, size) = match price_tick_type {
            TickType::Bid | TickType::DelayedBid => (&mut self.bid, &mut self.bid_size),
            TickType::Ask | TickType::DelayedAsk => (&mut self.ask, &mut self.ask_size),
            _ => return,
        };
        *price = None;
        *size = None;
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

/// Whether a bid or ask price tick is TWS's no-quote placeholder: size `0` with a price of `0` or
/// below, as seen on the wire (`docs/wire-facts/market-data.md#no-quote-sides`).
fn reports_no_quote(price: f64, size: f64) -> bool {
    size == 0.0 && price <= 0.0
}
