# Generic Tick Types for IB API

## Overview
Generic tick types are used with `reqMktData` to request specific market data fields. These are passed as a comma-separated string in the `genericTickList` parameter.

## Generic Tick Type IDs

### Real-Time Volume and Trade Data
- **100** - Option Volume (currently for stocks)
- **101** - Option Open Interest (currently for stocks) 
- **104** - Historical Volatility (currently for stocks)
- **105** - Average Option Volume (currently for stocks)
- **106** - Option Implied Volatility (currently for stocks)
- **162** - Index Future Premium
- **165** - Miscellaneous Stats
- **221** - Mark Price (used in P&L calculations)
- **225** - Auction values (volume, price and imbalance)
- **233** - RTVolume - Last trade price, last trade size, last trade time, total volume, VWAP, and single trade flag
- **236** - Shortable
- **256** - Inventory
- **258** - Fundamental Ratios
- **411** - Real-time Historical Volatility
- **456** - IBDividends

### News and Fundamentals
- **292** - News tick
- **293** - RT Trade Volume
- **294** - RT Historical Volatility
- **295** - RT Option Volume
- **318** - Last RTH Trade

### Additional Market Data
- **370** - Participation Monitor
- **375** - CTA ETF
- **377** - CTA ETF
- **381** - IB Rate
- **384** - RFQTQ
- **387** - DMM
- **388** - Issuer Fundamentals
- **391** - IB Warrant Impact Multiplier
- **407** - Futures Open Interest
- **428** - Monetary Close Price
- **439** - MonitorTickSize
- **459** - RTCLOSE
- **460** - Bond Factor Multiplier
- **499** - Fee and Rebate Rate
- **506** - Midpoint
- **511** - Trade Count
- **512** - Trade Rate
- **513** - Volume Rate  
- **514** - Last RTH Trade
- **515** - RT Historical Volatility
- **516** - IB Dividends
- **517** - Bond Coupon Rate
- **518** - Bond Price Statistics
- **519** - AGG GROUP
- **545** - Short-Term Volume 3 Minutes
- **546** - Short-Term Volume 5 Minutes
- **547** - Short-Term Volume 10 Minutes
- **548** - Futures Open Interest Change
- **572** - Average Daily Volume 21 Days
- **573** - Average Daily Volume 63 Days
- **574** - Average Daily Volume 126 Days
- **575** - Average Daily Volume 252 Days
- **576** - ETF NAV Close
- **577** - ETF NAV Prior Close
- **578** - ETF NAV Bid
- **579** - ETF NAV Ask
- **580** - ETF NAV Last
- **581** - ETF Frozen NAV Last
- **582** - ETF NAV High
- **583** - ETF NAV Low
- **584** - Social Sentiment
- **585** - Estimated IPO - Midpoint
- **586** - Final IPO Price
- **587** - Auction Strategy
- **588** - Delayed Bid
- **589** - Delayed Ask
- **590** - Delayed Last
- **591** - Delayed High
- **592** - Delayed Low
- **593** - Delayed Close
- **594** - Delayed Open
- **595** - RT TrdVolume

### Regulatory and Credit
- **619** - Creditman Mark Price
- **620** - Creditman Slow Mark Price
- **621** - Delayed Bid Option
- **622** - Delayed Ask Option
- **623** - Delayed Last Option
- **624** - Delayed Model Option
- **637** - Last Exch
- **638** - Last Reg Time
- **645** - Futures Open Interest
- **646** - Future's Block Trades
- **658** - Smart Components
- **662** - Trade Exchange
- **663** - Trade Currency

## Usage Example

```rust
// Request real-time volume and shortable data
client.req_mkt_data(
    req_id,
    &contract,
    "233,236", // RTVolume and Shortable
    false,
    false,
    vec![]
)?;
```

## Implementation Notes

### Rust Enum Design
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum GenericTickType {
    OptionVolume = 100,
    OptionOpenInterest = 101,
    HistoricalVolatility = 104,
    AverageOptionVolume = 105,
    OptionImpliedVolatility = 106,
    IndexFuturePremium = 162,
    MiscellaneousStats = 165,
    MarkPrice = 221,
    AuctionValues = 225,
    RTVolume = 233,
    Shortable = 236,
    Inventory = 256,
    FundamentalRatios = 258,
    NewsTick = 292,
    RTTradeVolume = 293,
    RTHistoricalVolatility = 294,
    RTOptionVolume = 295,
    LastRTHTrade = 318,
    ParticipationMonitor = 370,
    RealTimeHistoricalVolatility = 411,
    FuturesOpenInterest = 407,
    MonetaryClosePrice = 428,
    BondFactorMultiplier = 460,
    FeeAndRebateRate = 499,
    Midpoint = 506,
    TradeCount = 511,
    TradeRate = 512,
    VolumeRate = 513,
    ShortTermVolume3Min = 545,
    ShortTermVolume5Min = 546,
    ShortTermVolume10Min = 547,
    AverageDailyVolume21Days = 572,
    AverageDailyVolume63Days = 573,
    AverageDailyVolume126Days = 574,
    AverageDailyVolume252Days = 575,
    ETFNavClose = 576,
    ETFNavPriorClose = 577,
    ETFNavBid = 578,
    ETFNavAsk = 579,
    ETFNavLast = 580,
    ETFFrozenNavLast = 581,
    ETFNavHigh = 582,
    ETFNavLow = 583,
    SocialSentiment = 584,
    EstimatedIPOMidpoint = 585,
    FinalIPOPrice = 586,
    DelayedBid = 588,
    DelayedAsk = 589,
    DelayedLast = 590,
    DelayedHigh = 591,
    DelayedLow = 592,
    DelayedClose = 593,
    DelayedOpen = 594,
    CreditmanMarkPrice = 619,
    CreditmanSlowMarkPrice = 620,
    DelayedBidOption = 621,
    DelayedAskOption = 622,
    DelayedLastOption = 623,
    DelayedModelOption = 624,
    LastExch = 637,
    LastRegTime = 638,
    FuturesBlockTrades = 646,
    SmartComponents = 658,
    TradeExchange = 662,
    TradeCurrency = 663,
}

impl GenericTickType {
    pub fn to_string(&self) -> String {
        (*self as u16).to_string()
    }
}
```

### Alternative Constants Design
```rust
pub mod generic_ticks {
    pub const OPTION_VOLUME: u16 = 100;
    pub const OPTION_OPEN_INTEREST: u16 = 101;
    pub const HISTORICAL_VOLATILITY: u16 = 104;
    pub const AVERAGE_OPTION_VOLUME: u16 = 105;
    pub const OPTION_IMPLIED_VOLATILITY: u16 = 106;
    pub const INDEX_FUTURE_PREMIUM: u16 = 162;
    pub const MISCELLANEOUS_STATS: u16 = 165;
    pub const MARK_PRICE: u16 = 221;
    pub const AUCTION_VALUES: u16 = 225;
    pub const RT_VOLUME: u16 = 233;
    pub const SHORTABLE: u16 = 236;
    // ... etc
}
```

## Common Combinations

- **Real-time trading data**: "233" (RTVolume)
- **Options data**: "100,101,104,105,106" (volumes, OI, volatilities)
- **ETF data**: "576,577,578,579,580" (NAV values)
- **Delayed data**: "588,589,590,591,592,593,594" (all delayed fields)
- **Short selling**: "236" (Shortable)
- **Fundamentals**: "258,388" (ratios and issuer fundamentals)