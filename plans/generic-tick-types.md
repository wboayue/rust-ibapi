# Generic Tick Types for IB API

## Overview

`reqMktData` accepts a `genericTickList` parameter — a comma-separated string of
**generic tick request IDs**. Each ID subscribes to one or more *received* tick
types that the EWrapper then delivers via `tickPrice` / `tickSize` / `tickString` /
`tickGeneric` callbacks.

**Important:** generic-tick-request IDs are **not** the same as received tick IDs.

- **Generic tick request IDs** (this file): values like `100`, `233`, `236` —
  passed in `genericTickList`. Documented at
  <https://interactivebrokers.github.io/tws-api/tick_types.html> in the
  *Generic Tick Required* column.
- **Received tick IDs**: values like `0` (BidSize), `48` (RT_VOLUME), `86`
  (FUTURES_OPEN_INTEREST). These are field IDs in inbound messages and are
  defined in
  `/Users/wboayue/projects/tws-api/source/csharpclient/client/TickType.cs`.

Many tick types are delivered by default and require no entry in
`genericTickList`. Only the IDs below opt in to additional data.

## Current State (rust-ibapi)

Generic ticks are passed as `&[&str]` and joined with commas in
`market_data/realtime/common/encoders.rs`. No typed surface exists; callers
write raw numeric strings (e.g. `&["100", "101", "104", "106"]`). This task
adds a typed API and constant identifiers so callers don't memorize IDs.

## Generic Tick Request IDs

Verified against the IB docs page (and cross-checked against
`TickType.cs` for what each ID delivers).

| Generic ID | Subscribes to (received tick name → ID) | Description |
|-----------:|----------------------------------------|-------------|
| 100 | Option Call Volume (29), Option Put Volume (30) | Daily call/put option volume (currently for stocks) |
| 101 | Option Call Open Interest (27), Option Put Open Interest (28) | Call/put option open interest (currently for stocks) |
| 104 | Option Historical Volatility (23) | 30-day historical volatility (currently for stocks) |
| 105 | Average Option Volume (87) | Average volume of the corresponding option contracts |
| 106 | Option Implied Volatility (24) | IB's 30-day implied-volatility prediction |
| 162 | Index Future Premium (31) | Points the index future is over the cash index |
| 165 | Low/High 13W (15,16), Low/High 26W (17,18), Low/High 52W (19,20), Avg Volume (21) | Misc stats: weekly price ranges + 90-day average volume (stocks only) |
| 225 | Auction Volume (34), Auction Price (35), Auction Imbalance (36), Regulatory Imbalance (61) | Auction & regulatory imbalance values |
| 232 | Mark Price (37) | Theoretical calculated value used in P&L |
| 233 | RT Volume (48) | Time & Sales: last trade price/size/time, total volume, VWAP, single-trade flag (incl. unreportable trades) |
| 236 | Shortable (46), Shortable Shares (89) | Shortability level + shares available to short |
| 292 | News (62) | Contract news feed |
| 293 | Trade Count (54) | Trade count for the day |
| 294 | Trade Rate (55) | Trades per minute |
| 295 | Volume Rate (56) | Volume per minute |
| 318 | Last RTH Trade (57) | Last regular-trading-hours traded price |
| 375 | RT Trade Volume (77) | Time & Sales excluding unreportable trades |
| 411 | RT Historical Volatility (58) | 30-day real-time historical volatility |
| 456 | IB Dividends (59) | Past/future 12-month dividend sums + next dividend date/amount |
| 460 | Bond Factor Multiplier (60) | Ratio of current bond principal to original principal |
| 576 | ETF Nav Bid (94) | Bid price of ETF's Net Asset Value |
| 577 | ETF Nav Last (96) | Last price of ETF's Net Asset Value |
| 578 | ETF Nav Frozen Last (97) | Frozen Last price of ETF's NAV |
| 586 | Estimated IPO - Midpoint (101), Final IPO Price (102) | IPO pricing data |
| 588 | Futures Open Interest (86) | Total outstanding futures contracts |
| 595 | Short-Term Volume 3/5/10 Min (63,64,65) | Past 3/5/10-minute volume (stocks only) |
| 614 | ETF Nav High (98), ETF Nav Low (99) | High/Low NAV prices for the day |
| 619 | Creditman Slow Mark Price (79) | Slower mark-price update used in system calculations |
| 623 | ETF Nav Frozen Last (97) | (Same received tick as 578; documented under both) |

### Notes

- Tick `22` (OpenInterest) is documented as deprecated and is delivered by
  default; no generic ID requests it.
- "Tick `258` (Fundamental Ratios)" appears in older third-party docs and the
  prior version of this file but is **not** present on the current IB docs
  page — leave it out unless we can confirm it from a captured wire response.
- Delayed ticks (received IDs 66–76, 80–83, 88, 90, 103) are returned
  automatically when market-data type is set to delayed via `reqMarketDataType`;
  there is no generic ID for them.
- Many entries from the previous version of this file (256, 370, 377, 381, 384,
  387, 388, 391, 407, 428, 439, 459, 499, 506, 511–519, 545–548, 572–575,
  579–584, 587, 589–594, 620–624, 637, 638, 645, 646, 658, 662, 663) are
  either received tick IDs or values not documented as generic ticks. They
  were removed.

## Usage Example (current)

```rust
// Current API: raw numeric strings.
let subscription = client
    .market_data(&contract)
    .generic_ticks(&["233", "236"])  // RT Volume + Shortable
    .subscribe()?;
```

## Proposed Typed API

### Constants module (recommended)

Lightweight, no enum boilerplate, callers can still pass arbitrary IDs if a
new one ships before the crate is updated.

```rust
pub mod generic_tick {
    pub const OPTION_VOLUME: &str = "100";
    pub const OPTION_OPEN_INTEREST: &str = "101";
    pub const OPTION_HISTORICAL_VOLATILITY: &str = "104";
    pub const AVERAGE_OPTION_VOLUME: &str = "105";
    pub const OPTION_IMPLIED_VOLATILITY: &str = "106";
    pub const INDEX_FUTURE_PREMIUM: &str = "162";
    pub const MISC_STATS: &str = "165";
    pub const AUCTION_VALUES: &str = "225";
    pub const MARK_PRICE: &str = "232";
    pub const RT_VOLUME: &str = "233";
    pub const SHORTABLE: &str = "236";
    pub const NEWS: &str = "292";
    pub const TRADE_COUNT: &str = "293";
    pub const TRADE_RATE: &str = "294";
    pub const VOLUME_RATE: &str = "295";
    pub const LAST_RTH_TRADE: &str = "318";
    pub const RT_TRADE_VOLUME: &str = "375";
    pub const RT_HISTORICAL_VOLATILITY: &str = "411";
    pub const IB_DIVIDENDS: &str = "456";
    pub const BOND_FACTOR_MULTIPLIER: &str = "460";
    pub const ETF_NAV_BID: &str = "576";
    pub const ETF_NAV_LAST: &str = "577";
    pub const ETF_NAV_FROZEN_LAST: &str = "578";
    pub const IPO_PRICES: &str = "586";
    pub const FUTURES_OPEN_INTEREST: &str = "588";
    pub const SHORT_TERM_VOLUME: &str = "595";
    pub const ETF_NAV_HIGH_LOW: &str = "614";
    pub const CREDITMAN_SLOW_MARK_PRICE: &str = "619";
}

// Caller:
client.market_data(&contract)
    .generic_ticks(&[generic_tick::RT_VOLUME, generic_tick::SHORTABLE])
    .subscribe()?;
```

### Alternative: typed enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum GenericTickType {
    OptionVolume = 100,
    OptionOpenInterest = 101,
    OptionHistoricalVolatility = 104,
    AverageOptionVolume = 105,
    OptionImpliedVolatility = 106,
    IndexFuturePremium = 162,
    MiscStats = 165,
    AuctionValues = 225,
    MarkPrice = 232,
    RTVolume = 233,
    Shortable = 236,
    News = 292,
    TradeCount = 293,
    TradeRate = 294,
    VolumeRate = 295,
    LastRTHTrade = 318,
    RTTradeVolume = 375,
    RTHistoricalVolatility = 411,
    IBDividends = 456,
    BondFactorMultiplier = 460,
    ETFNavBid = 576,
    ETFNavLast = 577,
    ETFNavFrozenLast = 578,
    IPOPrices = 586,
    FuturesOpenInterest = 588,
    ShortTermVolume = 595,
    ETFNavHighLow = 614,
    CreditmanSlowMarkPrice = 619,
}

impl std::fmt::Display for GenericTickType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u16)
    }
}
```

Trade-off: enum is type-safe but breaks if IB adds a new generic ID before
we ship a release. Constants accept raw `&str` so callers always have an
escape hatch. The builder should accept either via something like
`generic_ticks<S: AsRef<str>>(self, ticks: &[S])` (already the shape in
`testdata/builders/market_data.rs`).

## Common Combinations

- **Time & Sales (incl. unreportable)**: `"233"` (RT Volume)
- **Time & Sales (excl. unreportable)**: `"375"` (RT Trade Volume)
- **Options (stock-level)**: `"100,101,104,105,106"` (volumes, OI, vols)
- **ETF NAV (full set)**: `"576,577,578,614,623"` (bid/last/frozen + high/low)
- **Shortability**: `"236"`
- **Per-minute activity**: `"293,294,295"` (count, trade rate, volume rate)
- **News**: `"292"`
- **Mark / regulatory**: `"232,225"` (mark price + auction & reg imbalance)

## Implementation Plan

1. Add `src/market_data/realtime/generic_tick.rs` with the constants module
   (start there; enum can come later if there's demand).
2. Re-export from `src/market_data/realtime/mod.rs` so callers get
   `ibapi::market_data::realtime::generic_tick::RT_VOLUME`.
3. Update doc-comment example on the `generic_ticks(...)` builder method to
   show the constant form. The builder signature stays generic over `AsRef<str>`.
4. Add a sibling `generic_tick_tests.rs` asserting each constant matches the
   numeric ID listed above (regression guard if someone re-types a digit).
5. Update `docs/api-patterns.md` with a short subsection cross-linking the
   constants and the IB docs page.

## References

- IB docs: <https://interactivebrokers.github.io/tws-api/tick_types.html>
- C# received-tick IDs: `/Users/wboayue/projects/tws-api/source/csharpclient/client/TickType.cs`
- C# `EClient.reqMktData` (encoding of `genericTickList`): `EClient.cs`
