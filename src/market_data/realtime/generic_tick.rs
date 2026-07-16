//! Named constants for IB *generic tick request IDs*.
//!
//! These are the values you pass via the `genericTickList` parameter on
//! `reqMktData` (the [`MarketDataBuilder::generic_ticks`] /
//! [`MarketDataBuilder::add_generic_tick`] methods). Each ID subscribes the
//! market-data stream to one or more *received* tick types that arrive on the
//! `tickPrice` / `tickSize` / `tickString` / `tickGeneric` callbacks.
//!
//! **Generic tick request IDs are not received tick IDs.** Received tick IDs
//! (`BID_SIZE` = 0, `RT_VOLUME` = 48, `FUTURES_OPEN_INTEREST` = 86, …) are
//! field IDs on inbound messages and live in
//! [`contracts::tick_types::TickType`](crate::contracts::tick_types::TickType).
//! The doc-comment on each constant below names the received-tick types it
//! subscribes to using the IB API's canonical upper-snake-case spelling.
//!
//! Many tick types are delivered by default and require no entry in
//! `genericTickList`. Only the IDs below opt in to additional data.
//!
//! # Example
//!
//! ```no_run
//! use ibapi::market_data::realtime::generic_tick;
//!
//! # #[cfg(feature = "sync")]
//! # {
//! use ibapi::client::blocking::Client;
//! use ibapi::contracts::Contract;
//!
//! let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
//! let contract = Contract::stock("AAPL").build();
//!
//! let subscription = client
//!     .market_data(&contract)
//!     .generic_ticks(&[generic_tick::RT_VOLUME, generic_tick::SHORTABLE])
//!     .subscribe()
//!     .expect("subscription failed");
//! # let _ = subscription;
//! # }
//! ```
//!
//! # References
//!
//! - IB docs: <https://interactivebrokers.github.io/tws-api/tick_types.html>
//!   (see the *Generic Tick Required* column).
//!
//! [`MarketDataBuilder::generic_ticks`]: crate::market_data::builder::MarketDataBuilder::generic_ticks
//! [`MarketDataBuilder::add_generic_tick`]: crate::market_data::builder::MarketDataBuilder::add_generic_tick

/// `100` — Daily call/put option volume (currently for stocks).
///
/// Delivers received ticks `OPTION_CALL_VOLUME` (29) and `OPTION_PUT_VOLUME` (30).
pub const OPTION_VOLUME: &str = "100";

/// `101` — Call/put option open interest (currently for stocks).
///
/// Delivers received ticks `OPTION_CALL_OPEN_INTEREST` (27) and
/// `OPTION_PUT_OPEN_INTEREST` (28).
pub const OPTION_OPEN_INTEREST: &str = "101";

/// `104` — 30-day historical volatility (currently for stocks).
///
/// Delivers received tick `OPTION_HISTORICAL_VOL` (23).
pub const OPTION_HISTORICAL_VOLATILITY: &str = "104";

/// `105` — Average volume of the corresponding option contracts.
///
/// Delivers received tick `AVG_OPT_VOLUME` (87).
pub const AVERAGE_OPTION_VOLUME: &str = "105";

/// `106` — IB's 30-day implied-volatility prediction (currently for stocks).
///
/// Delivers received tick `OPTION_IMPLIED_VOL` (24).
pub const OPTION_IMPLIED_VOLATILITY: &str = "106";

/// `162` — Points the index future is over the cash index.
///
/// Delivers received tick `INDEX_FUTURE_PREMIUM` (31).
pub const INDEX_FUTURE_PREMIUM: &str = "162";

/// `165` — Miscellaneous stats: weekly price ranges + 90-day average volume
/// (stocks only).
///
/// Delivers received ticks `LOW_13_WEEK` (15), `HIGH_13_WEEK` (16),
/// `LOW_26_WEEK` (17), `HIGH_26_WEEK` (18), `LOW_52_WEEK` (19),
/// `HIGH_52_WEEK` (20), and `AVG_VOLUME` (21).
pub const MISC_STATS: &str = "165";

/// `225` — Auction & regulatory imbalance values.
///
/// Delivers received ticks `AUCTION_VOLUME` (34), `AUCTION_PRICE` (35),
/// `AUCTION_IMBALANCE` (36), and `REGULATORY_IMBALANCE` (61).
pub const AUCTION_VALUES: &str = "225";

/// `232` — Theoretical mark price used in P&L.
///
/// Delivers received tick `MARK_PRICE` (37).
pub const MARK_PRICE: &str = "232";

/// `233` — Time & Sales (last trade price/size/time, total volume, VWAP,
/// single-trade flag), including unreportable trades.
///
/// Delivers received tick `RT_VOLUME` (48).
pub const RT_VOLUME: &str = "233";

/// `236` — Shortability level + shares available to short.
///
/// Delivers received ticks `SHORTABLE` (46) and `SHORTABLE_SHARES` (89).
pub const SHORTABLE: &str = "236";

/// `292` — Contract news feed.
///
/// Delivers received tick `NEWS_TICK` (62).
pub const NEWS: &str = "292";

/// `293` — Trade count for the day.
///
/// Delivers received tick `TRADE_COUNT` (54).
pub const TRADE_COUNT: &str = "293";

/// `294` — Trades per minute.
///
/// Delivers received tick `TRADE_RATE` (55).
pub const TRADE_RATE: &str = "294";

/// `295` — Volume per minute.
///
/// Delivers received tick `VOLUME_RATE` (56).
pub const VOLUME_RATE: &str = "295";

/// `318` — Last regular-trading-hours traded price.
///
/// Delivers received tick `LAST_RTH_TRADE` (57).
pub const LAST_RTH_TRADE: &str = "318";

/// `375` — Time & Sales excluding unreportable trades.
///
/// Delivers received tick `RT_TRD_VOLUME` (77).
pub const RT_TRADE_VOLUME: &str = "375";

/// `411` — 30-day real-time historical volatility.
///
/// Delivers received tick `RT_HISTORICAL_VOL` (58).
pub const RT_HISTORICAL_VOLATILITY: &str = "411";

/// `456` — Past/future 12-month dividend sums + next dividend date/amount.
///
/// Delivers received tick `IB_DIVIDENDS` (59).
pub const IB_DIVIDENDS: &str = "456";

/// `460` — Ratio of current bond principal to original principal.
///
/// Delivers received tick `BOND_FACTOR_MULTIPLIER` (60).
pub const BOND_FACTOR_MULTIPLIER: &str = "460";

/// `576` — Bid price of ETF's Net Asset Value.
///
/// Delivers received tick `ETF_NAV_BID` (94).
pub const ETF_NAV_BID: &str = "576";

/// `577` — Last price of ETF's Net Asset Value.
///
/// Delivers received tick `ETF_NAV_LAST` (96).
pub const ETF_NAV_LAST: &str = "577";

/// `578` — Frozen last price of ETF's NAV.
///
/// Delivers received tick `ETF_FROZEN_NAV_LAST` (97).
pub const ETF_NAV_FROZEN_LAST: &str = "578";

/// `586` — IPO pricing data.
///
/// Delivers received ticks `ESTIMATED_IPO_MIDPOINT` (101) and
/// `FINAL_IPO_LAST` (102).
pub const IPO_PRICES: &str = "586";

/// `588` — Total outstanding futures contracts.
///
/// Delivers received tick `FUTURES_OPEN_INTEREST` (86).
pub const FUTURES_OPEN_INTEREST: &str = "588";

/// `595` — Past 3/5/10-minute volume (stocks only).
///
/// Delivers received ticks `SHORT_TERM_VOLUME_3_MIN` (63),
/// `SHORT_TERM_VOLUME_5_MIN` (64), and `SHORT_TERM_VOLUME_10_MIN` (65).
pub const SHORT_TERM_VOLUME: &str = "595";

/// `614` — High/Low NAV prices for the day.
///
/// Delivers received ticks `ETF_NAV_HIGH` (98) and `ETF_NAV_LOW` (99).
pub const ETF_NAV_HIGH_LOW: &str = "614";

/// `619` — Slower mark-price update used in system calculations.
///
/// Delivers received tick `CREDITMAN_SLOW_MARK_PRICE` (79).
pub const CREDITMAN_SLOW_MARK_PRICE: &str = "619";

/// `787` — Odd-lot bid/ask quotes.
///
/// Delivers received ticks `ODD_LOT_BID` (105), `ODD_LOT_ASK` (106),
/// `ODD_LOT_BID_SIZE` (107), `ODD_LOT_ASK_SIZE` (108), `ODD_LOT_BID_EXCH` (109),
/// and `ODD_LOT_ASK_EXCH` (110).
///
/// Requires TWS/Gateway server version 225 (`ODD_LOT_BID_ASK_QUOTES`) or later.
pub const ODD_LOT: &str = "787";

#[cfg(test)]
#[path = "generic_tick_tests.rs"]
mod tests;
