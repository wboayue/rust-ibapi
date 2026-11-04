use std::fmt;

/// Generates [`TickType`] and its conversions from one table, so the wire code, C# field name and
/// display name of a tick can't drift apart. Each row is
/// `Variant = code, "csharpFieldName", "Display Name";` under the variant's doc comment.
macro_rules! tick_types {
    ($( $(#[$meta:meta])* $variant:ident = $code:literal, $name:literal, $display:literal; )*) => {
        /// Market data tick types available from the TWS API.
        ///
        /// These represent different types of market data that can be requested
        /// and received from Interactive Brokers. Each tick type corresponds to
        /// a specific piece of market information like bid, ask, last trade, volume, etc.
        #[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum TickType {
            $(
                $(#[$meta])*
                #[doc = concat!("\n\nWire code `", stringify!($code), "`.")]
                $variant,
            )*
            /// Tick id not modeled by this version of the API, carried as received.
            Unknown(i32),
        }

        impl From<i32> for TickType {
            fn from(value: i32) -> Self {
                match value {
                    $( $code => Self::$variant, )*
                    code => Self::Unknown(code),
                }
            }
        }

        impl From<TickType> for i32 {
            fn from(value: TickType) -> i32 {
                match value {
                    $( TickType::$variant => $code, )*
                    TickType::Unknown(code) => code,
                }
            }
        }

        impl TickType {
            /// Looks up a tick type by its TWS API field name, as the reference client's
            /// `TickType.getField` spells it (`"bidSize"`, `"bidPrice"`, `"trades/min"`).
            /// Matching is exact and case-sensitive; `None` for any other name.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $( $name => Some(Self::$variant), )*
                    _ => None,
                }
            }

            /// The TWS API field name of this tick type, the inverse of
            /// [`from_name`](Self::from_name). `None` for [`Unknown`](Self::Unknown).
            pub fn name(&self) -> Option<&'static str> {
                match self {
                    $( Self::$variant => Some($name), )*
                    Self::Unknown(_) => None,
                }
            }
        }

        impl fmt::Display for TickType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $( Self::$variant => f.write_str($display), )*
                    Self::Unknown(code) => write!(f, "Unknown({code})"),
                }
            }
        }
    };
}

tick_types! {
    /// Number of contracts or shares offered at the bid price.
    BidSize = 0, "bidSize", "Bid Size";
    /// Highest price a buyer is willing to pay.
    Bid = 1, "bidPrice", "Bid";
    /// Lowest price a seller is willing to accept.
    Ask = 2, "askPrice", "Ask";
    /// Number of contracts or shares offered at the ask price.
    AskSize = 3, "askSize", "Ask Size";
    /// Price of the last trade.
    Last = 4, "lastPrice", "Last";
    /// Number of contracts or shares traded in the last trade.
    LastSize = 5, "lastSize", "Last Size";
    /// Highest price of the day.
    High = 6, "high", "High";
    /// Lowest price of the day.
    Low = 7, "low", "Low";
    /// Total trading volume for the day.
    Volume = 8, "volume", "Volume";
    /// Previous day's closing price.
    Close = 9, "close", "Close";
    /// Bid price for options.
    BidOption = 10, "bidOptComp", "Bid Option";
    /// Ask price for options.
    AskOption = 11, "askOptComp", "Ask Option";
    /// Last traded price for options.
    LastOption = 12, "lastOptComp", "Last Option";
    /// Model-based option price.
    ModelOption = 13, "modelOptComp", "Model Option";
    /// Opening price of the day.
    Open = 14, "open", "Open";
    /// Lowest price in the last 13 weeks.
    Low13Week = 15, "13WeekLow", "13 Week Low";
    /// Highest price in the last 13 weeks.
    High13Week = 16, "13WeekHigh", "13 Week High";
    /// Lowest price in the last 26 weeks.
    Low26Week = 17, "26WeekLow", "26 Week Low";
    /// Highest price in the last 26 weeks.
    High26Week = 18, "26WeekHigh", "26 Week High";
    /// Lowest price in the last 52 weeks.
    Low52Week = 19, "52WeekLow", "52 Week Low";
    /// Highest price in the last 52 weeks.
    High52Week = 20, "52WeekHigh", "52 Week High";
    /// Average daily trading volume.
    AvgVolume = 21, "AvgVolume", "Average Volume";
    /// Total number of outstanding contracts.
    OpenInterest = 22, "OpenInterest", "Open Interest";
    /// Historical volatility for options.
    OptionHistoricalVol = 23, "OptionHistoricalVolatility", "Option Historical Volatility";
    /// Implied volatility for options.
    OptionImpliedVol = 24, "OptionImpliedVolatility", "Option Implied Volatility";
    /// Exchange code from which the option bid quote originated.
    OptionBidExch = 25, "OptionBidExchStr", "Option Bid Exchange";
    /// Exchange code from which the option ask quote originated.
    OptionAskExch = 26, "OptionAskExchStr", "Option Ask Exchange";
    /// Current open interest for call options.
    OptionCallOpenInterest = 27, "OptionCallOpenInterest", "Option Call Open Interest";
    /// Current open interest for put options.
    OptionPutOpenInterest = 28, "OptionPutOpenInterest", "Option Put Open Interest";
    /// Trading volume for call options.
    OptionCallVolume = 29, "OptionCallVolume", "Option Call Volume";
    /// Trading volume for put options.
    OptionPutVolume = 30, "OptionPutVolume", "Option Put Volume";
    /// Premium of the index future over fair value.
    IndexFuturePremium = 31, "IndexFuturePremium", "Index Future Premium";
    /// Exchange code supplying the top-of-book bid.
    BidExch = 32, "bidExch", "Bid Exchange";
    /// Exchange code supplying the top-of-book ask.
    AskExch = 33, "askExch", "Ask Exchange";
    /// Shares/contracts offered during the auction.
    AuctionVolume = 34, "auctionVolume", "Auction Volume";
    /// Indicative auction clearing price.
    AuctionPrice = 35, "auctionPrice", "Auction Price";
    /// Imbalance between buy and sell interest in the auction.
    AuctionImbalance = 36, "auctionImbalance", "Auction Imbalance";
    /// Mark price (used for margining in futures).
    MarkPrice = 37, "markPrice", "Mark Price";
    /// Bid-side exchange for EFP (Exchange for Physical) computations.
    BidEfpComputation = 38, "bidEFP", "Bid EFP Computation";
    /// Ask-side exchange for EFP computations.
    AskEfpComputation = 39, "askEFP", "Ask EFP Computation";
    /// Last trade exchange for EFP computations.
    LastEfpComputation = 40, "lastEFP", "Last EFP Computation";
    /// Opening price exchange for EFP computations.
    OpenEfpComputation = 41, "openEFP", "Open EFP Computation";
    /// High price exchange for EFP computations.
    HighEfpComputation = 42, "highEFP", "High EFP Computation";
    /// Low price exchange for EFP computations.
    LowEfpComputation = 43, "lowEFP", "Low EFP Computation";
    /// Closing price exchange for EFP computations.
    CloseEfpComputation = 44, "closeEFP", "Close EFP Computation";
    /// Timestamp (epoch seconds) for the last tick.
    LastTimestamp = 45, "lastTimestamp", "Last Timestamp";
    /// Number of shares available for shorting.
    Shortable = 46, "shortable", "Shortable";
    // id 47 (FundamentalRatios) removed: IBKR removed the fundamental-data
    // feature in TWS 10.47; id 47 now decodes to `Unknown`.
    /// Real-time consolidated volume message.
    RtVolume = 48, "RTVolume", "RT Volume";
    /// Indicates if trading is halted (0 = not halted, 1 = halted).
    Halted = 49, "halted", "Halted";
    /// Yield computed from the bid price (fixed income).
    BidYield = 50, "bidYield", "Bid Yield";
    /// Yield computed from the ask price (fixed income).
    AskYield = 51, "askYield", "Ask Yield";
    /// Yield of the last trade (fixed income).
    LastYield = 52, "lastYield", "Last Yield";
    /// Custom option computation values supplied by IB.
    CustOptionComputation = 53, "custOptComp", "Custom Option Computation";
    /// Number of trades over the reporting interval.
    TradeCount = 54, "trades", "Trade Count";
    /// Trades per minute over the interval.
    TradeRate = 55, "trades/min", "Trade Rate";
    /// Volume per minute over the interval.
    VolumeRate = 56, "volume/min", "Volume Rate";
    /// Last regular trading hours trade details.
    LastRthTrade = 57, "lastRTHTrade", "Last RTH Trade";
    /// Real-time historical volatility value.
    RtHistoricalVol = 58, "RTHistoricalVol", "RT Historical Volatility";
    /// Projected dividend information.
    IbDividends = 59, "IBDividends", "IB Dividends";
    /// Factor multiplier used for bonds.
    BondFactorMultiplier = 60, "bondFactorMultiplier", "Bond Factor Multiplier";
    /// Regulatory imbalance indicator from exchanges.
    RegulatoryImbalance = 61, "regulatoryImbalance", "Regulatory Imbalance";
    /// Exchange-issued news headline.
    NewsTick = 62, "newsTick", "News Tick";
    /// Short-term volume averaged over 3 minutes.
    ShortTermVolume3Min = 63, "shortTermVolume3Min", "Short Term Volume 3 Min";
    /// Short-term volume averaged over 5 minutes.
    ShortTermVolume5Min = 64, "shortTermVolume5Min", "Short Term Volume 5 Min";
    /// Short-term volume averaged over 10 minutes.
    ShortTermVolume10Min = 65, "shortTermVolume10Min", "Short Term Volume 10 Min";
    /// Delayed bid price.
    DelayedBid = 66, "delayedBid", "Delayed Bid";
    /// Delayed ask price.
    DelayedAsk = 67, "delayedAsk", "Delayed Ask";
    /// Delayed last traded price.
    DelayedLast = 68, "delayedLast", "Delayed Last";
    /// Delayed bid size.
    DelayedBidSize = 69, "delayedBidSize", "Delayed Bid Size";
    /// Delayed ask size.
    DelayedAskSize = 70, "delayedAskSize", "Delayed Ask Size";
    /// Delayed last trade size.
    DelayedLastSize = 71, "delayedLastSize", "Delayed Last Size";
    /// Delayed session high.
    DelayedHigh = 72, "delayedHigh", "Delayed High";
    /// Delayed session low.
    DelayedLow = 73, "delayedLow", "Delayed Low";
    /// Delayed volume.
    DelayedVolume = 74, "delayedVolume", "Delayed Volume";
    /// Delayed session close.
    DelayedClose = 75, "delayedClose", "Delayed Close";
    /// Delayed session open.
    DelayedOpen = 76, "delayedOpen", "Delayed Open";
    /// Real-time trade volume (aggregate).
    RtTrdVolume = 77, "rtTrdVolume", "RT Trade Volume";
    /// Credit manager mark price.
    CreditmanMarkPrice = 78, "creditmanMarkPrice", "Creditman Mark Price";
    /// Credit manager slow mark price.
    CreditmanSlowMarkPrice = 79, "creditmanSlowMarkPrice", "Creditman Slow Mark Price";
    /// Delayed option bid computation.
    DelayedBidOption = 80, "delayedBidOptComp", "Delayed Bid Option";
    /// Delayed option ask computation.
    DelayedAskOption = 81, "delayedAskOptComp", "Delayed Ask Option";
    /// Delayed option last computation.
    DelayedLastOption = 82, "delayedLastOptComp", "Delayed Last Option";
    /// Delayed option model price.
    DelayedModelOption = 83, "delayedModelOptComp", "Delayed Model Option";
    /// Exchange code for the last trade.
    LastExch = 84, "lastExchange", "Last Exchange";
    /// Time of the last regular trade.
    LastRegTime = 85, "lastRegTime", "Last Reg Time";
    /// Open interest for futures contracts.
    FuturesOpenInterest = 86, "futuresOpenInterest", "Futures Open Interest";
    /// Average option volume over 90 days.
    AvgOptVolume = 87, "avgOptVolume", "Average Option Volume";
    /// Timestamp associated with delayed last price.
    DelayedLastTimestamp = 88, "delayedLastTimestamp", "Delayed Last Timestamp";
    /// Number of shares available to short.
    ShortableShares = 89, "shortableShares", "Shortable Shares";
    /// Delayed trading halt status.
    DelayedHalted = 90, "delayedHalted", "Delayed Halted";
    /// Reuters mutual fund indicative value.
    Reuters2MutualFunds = 91, "reuters2MutualFunds", "Reuters2 Mutual Funds";
    /// ETF net asset value (NAV) close.
    EtfNavClose = 92, "etfNavClose", "ETF NAV Close";
    /// Prior day ETF NAV close.
    EtfNavPriorClose = 93, "etfNavPriorClose", "ETF NAV Prior Close";
    /// ETF NAV bid.
    EtfNavBid = 94, "etfNavBid", "ETF NAV Bid";
    /// ETF NAV ask.
    EtfNavAsk = 95, "etfNavAsk", "ETF NAV Ask";
    /// ETF NAV last.
    EtfNavLast = 96, "etfNavLast", "ETF NAV Last";
    /// Frozen ETF NAV last value.
    EtfFrozenNavLast = 97, "etfFrozenNavLast", "ETF Frozen NAV Last";
    /// Intraday high estimate for ETF NAV.
    EtfNavHigh = 98, "etfNavHigh", "ETF NAV High";
    /// Intraday low estimate for ETF NAV.
    EtfNavLow = 99, "etfNavLow", "ETF NAV Low";
    /// Social market analytics sentiment score.
    SocialMarketAnalytics = 100, "socialMarketAnalytics", "Social Market Analytics";
    /// Estimated midpoint for an IPO.
    EstimatedIpoMidpoint = 101, "estimatedIPOMidpoint", "Estimated IPO Midpoint";
    /// Final pricing for an IPO.
    FinalIpoLast = 102, "finalIPOLast", "Final IPO Last";
    /// Delayed bid yield.
    DelayedYieldBid = 103, "delayedYieldBid", "Delayed Yield Bid";
    /// Delayed ask yield.
    DelayedYieldAsk = 104, "delayedYieldAsk", "Delayed Yield Ask";
    /// Odd-lot bid price.
    OddLotBid = 105, "oddLotBid", "Odd Lot Bid";
    /// Odd-lot ask price.
    OddLotAsk = 106, "oddLotAsk", "Odd Lot Ask";
    /// Odd-lot bid size.
    OddLotBidSize = 107, "oddLotBidSize", "Odd Lot Bid Size";
    /// Odd-lot ask size.
    OddLotAskSize = 108, "oddLotAskSize", "Odd Lot Ask Size";
    /// Exchange code supplying the odd-lot bid.
    OddLotBidExch = 109, "oddLotBidExch", "Odd Lot Bid Exchange";
    /// Exchange code supplying the odd-lot ask.
    OddLotAskExch = 110, "oddLotAskExch", "Odd Lot Ask Exchange";
    /// Implied bid price of a combo (BAG) contract.
    ComboImpliedBid = 112, "comboImpliedBid", "Combo Implied Bid";
    /// Implied ask price of a combo (BAG) contract.
    ComboImpliedAsk = 113, "comboImpliedAsk", "Combo Implied Ask";
}

/// `Unknown(-1)`: no tick type, the value `Unknown` had before 6.0. IB assigns no tick id `-1`.
impl Default for TickType {
    fn default() -> Self {
        Self::Unknown(-1)
    }
}

#[cfg(test)]
#[path = "tick_types_tests.rs"]
mod tests;
