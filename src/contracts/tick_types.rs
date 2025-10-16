use std::fmt;

/// Market data tick types available from the TWS API.
///
/// These represent different types of market data that can be requested
/// and received from Interactive Brokers. Each tick type corresponds to
/// a specific piece of market information like bid, ask, last trade, volume, etc.
#[derive(Debug, PartialEq, Default)]
pub enum TickType {
    /// Unknown or invalid tick type.
    #[default]
    Unknown = -1,
    /// Number of contracts or shares offered at the bid price.
    BidSize = 0,
    /// Highest price a buyer is willing to pay.
    Bid = 1,
    /// Lowest price a seller is willing to accept.
    Ask = 2,
    /// Number of contracts or shares offered at the ask price.
    AskSize = 3,
    /// Price of the last trade.
    Last = 4,
    /// Number of contracts or shares traded in the last trade.
    LastSize = 5,
    /// Highest price of the day.
    High = 6,
    /// Lowest price of the day.
    Low = 7,
    /// Total trading volume for the day.
    Volume = 8,
    /// Previous day's closing price.
    Close = 9,
    /// Bid price for options.
    BidOption = 10,
    /// Ask price for options.
    AskOption = 11,
    /// Last traded price for options.
    LastOption = 12,
    /// Model-based option price.
    ModelOption = 13,
    /// Opening price of the day.
    Open = 14,
    /// Lowest price in the last 13 weeks.
    Low13Week = 15,
    /// Highest price in the last 13 weeks.
    High13Week = 16,
    /// Lowest price in the last 26 weeks.
    Low26Week = 17,
    /// Highest price in the last 26 weeks.
    High26Week = 18,
    /// Lowest price in the last 52 weeks.
    Low52Week = 19,
    /// Highest price in the last 52 weeks.
    High52Week = 20,
    /// Average daily trading volume.
    AvgVolume = 21,
    /// Total number of outstanding contracts.
    OpenInterest = 22,
    /// Historical volatility for options.
    OptionHistoricalVol = 23,
    /// Implied volatility for options.
    OptionImpliedVol = 24,
    /// Exchange code from which the option bid quote originated.
    OptionBidExch = 25,
    /// Exchange code from which the option ask quote originated.
    OptionAskExch = 26,
    /// Current open interest for call options.
    OptionCallOpenInterest = 27,
    /// Current open interest for put options.
    OptionPutOpenInterest = 28,
    /// Trading volume for call options.
    OptionCallVolume = 29,
    /// Trading volume for put options.
    OptionPutVolume = 30,
    /// Premium of the index future over fair value.
    IndexFuturePremium = 31,
    /// Exchange code supplying the top-of-book bid.
    BidExch = 32,
    /// Exchange code supplying the top-of-book ask.
    AskExch = 33,
    /// Shares/contracts offered during the auction.
    AuctionVolume = 34,
    /// Indicative auction clearing price.
    AuctionPrice = 35,
    /// Imbalance between buy and sell interest in the auction.
    AuctionImbalance = 36,
    /// Mark price (used for margining in futures).
    MarkPrice = 37,
    /// Bid-side exchange for EFP (Exchange for Physical) computations.
    BidEfpComputation = 38,
    /// Ask-side exchange for EFP computations.
    AskEfpComputation = 39,
    /// Last trade exchange for EFP computations.
    LastEfpComputation = 40,
    /// Opening price exchange for EFP computations.
    OpenEfpComputation = 41,
    /// High price exchange for EFP computations.
    HighEfpComputation = 42,
    /// Low price exchange for EFP computations.
    LowEfpComputation = 43,
    /// Closing price exchange for EFP computations.
    CloseEfpComputation = 44,
    /// Timestamp (epoch seconds) for the last tick.
    LastTimestamp = 45,
    /// Number of shares available for shorting.
    Shortable = 46,
    /// Fundamental ratios snapshot (PE, EPS, etc.).
    FundamentalRatios = 47,
    /// Real-time consolidated volume message.
    RtVolume = 48,
    /// Indicates if trading is halted (0 = not halted, 1 = halted).
    Halted = 49,
    /// Yield computed from the bid price (fixed income).
    BidYield = 50,
    /// Yield computed from the ask price (fixed income).
    AskYield = 51,
    /// Yield of the last trade (fixed income).
    LastYield = 52,
    /// Custom option computation values supplied by IB.
    CustOptionComputation = 53,
    /// Number of trades over the reporting interval.
    TradeCount = 54,
    /// Trades per minute over the interval.
    TradeRate = 55,
    /// Volume per minute over the interval.
    VolumeRate = 56,
    /// Last regular trading hours trade details.
    LastRthTrade = 57,
    /// Real-time historical volatility value.
    RtHistoricalVol = 58,
    /// Projected dividend information.
    IbDividends = 59,
    /// Factor multiplier used for bonds.
    BondFactorMultiplier = 60,
    /// Regulatory imbalance indicator from exchanges.
    RegulatoryImbalance = 61,
    /// Exchange-issued news headline.
    NewsTick = 62,
    /// Short-term volume averaged over 3 minutes.
    ShortTermVolume3Min = 63,
    /// Short-term volume averaged over 5 minutes.
    ShortTermVolume5Min = 64,
    /// Short-term volume averaged over 10 minutes.
    ShortTermVolume10Min = 65,
    /// Delayed bid price.
    DelayedBid = 66,
    /// Delayed ask price.
    DelayedAsk = 67,
    /// Delayed last traded price.
    DelayedLast = 68,
    /// Delayed bid size.
    DelayedBidSize = 69,
    /// Delayed ask size.
    DelayedAskSize = 70,
    /// Delayed last trade size.
    DelayedLastSize = 71,
    /// Delayed session high.
    DelayedHigh = 72,
    /// Delayed session low.
    DelayedLow = 73,
    /// Delayed volume.
    DelayedVolume = 74,
    /// Delayed session close.
    DelayedClose = 75,
    /// Delayed session open.
    DelayedOpen = 76,
    /// Real-time trade volume (aggregate).
    RtTrdVolume = 77,
    /// Credit manager mark price.
    CreditmanMarkPrice = 78,
    /// Credit manager slow mark price.
    CreditmanSlowMarkPrice = 79,
    /// Delayed option bid computation.
    DelayedBidOption = 80,
    /// Delayed option ask computation.
    DelayedAskOption = 81,
    /// Delayed option last computation.
    DelayedLastOption = 82,
    /// Delayed option model price.
    DelayedModelOption = 83,
    /// Exchange code for the last trade.
    LastExch = 84,
    /// Time of the last regular trade.
    LastRegTime = 85,
    /// Open interest for futures contracts.
    FuturesOpenInterest = 86,
    /// Average option volume over 90 days.
    AvgOptVolume = 87,
    /// Timestamp associated with delayed last price.
    DelayedLastTimestamp = 88,
    /// Number of shares available to short.
    ShortableShares = 89,
    /// Delayed trading halt status.
    DelayedHalted = 90,
    /// Reuters mutual fund indicative value.
    Reuters2MutualFunds = 91,
    /// ETF net asset value (NAV) close.
    EtfNavClose = 92,
    /// Prior day ETF NAV close.
    EtfNavPriorClose = 93,
    /// ETF NAV bid.
    EtfNavBid = 94,
    /// ETF NAV ask.
    EtfNavAsk = 95,
    /// ETF NAV last.
    EtfNavLast = 96,
    /// Frozen ETF NAV last value.
    EtfFrozenNavLast = 97,
    /// Intraday high estimate for ETF NAV.
    EtfNavHigh = 98,
    /// Intraday low estimate for ETF NAV.
    EtfNavLow = 99,
    /// Social market analytics sentiment score.
    SocialMarketAnalytics = 100,
    /// Estimated midpoint for an IPO.
    EstimatedIpoMidpoint = 101,
    /// Final pricing for an IPO.
    FinalIpoLast = 102,
    /// Delayed bid yield.
    DelayedYieldBid = 103,
    /// Delayed ask yield.
    DelayedYieldAsk = 104,
}

impl From<i32> for TickType {
    fn from(value: i32) -> Self {
        match value {
            -1 => Self::Unknown,
            0 => Self::BidSize,
            1 => Self::Bid,
            2 => Self::Ask,
            3 => Self::AskSize,
            4 => Self::Last,
            5 => Self::LastSize,
            6 => Self::High,
            7 => Self::Low,
            8 => Self::Volume,
            9 => Self::Close,
            10 => Self::BidOption,
            11 => Self::AskOption,
            12 => Self::LastOption,
            13 => Self::ModelOption,
            14 => Self::Open,
            15 => Self::Low13Week,
            16 => Self::High13Week,
            17 => Self::Low26Week,
            18 => Self::High26Week,
            19 => Self::Low52Week,
            20 => Self::High52Week,
            21 => Self::AvgVolume,
            22 => Self::OpenInterest,
            23 => Self::OptionHistoricalVol,
            24 => Self::OptionImpliedVol,
            25 => Self::OptionBidExch,
            26 => Self::OptionAskExch,
            27 => Self::OptionCallOpenInterest,
            28 => Self::OptionPutOpenInterest,
            29 => Self::OptionCallVolume,
            30 => Self::OptionPutVolume,
            31 => Self::IndexFuturePremium,
            32 => Self::BidExch,
            33 => Self::AskExch,
            34 => Self::AuctionVolume,
            35 => Self::AuctionPrice,
            36 => Self::AuctionImbalance,
            37 => Self::MarkPrice,
            38 => Self::BidEfpComputation,
            39 => Self::AskEfpComputation,
            40 => Self::LastEfpComputation,
            41 => Self::OpenEfpComputation,
            42 => Self::HighEfpComputation,
            43 => Self::LowEfpComputation,
            44 => Self::CloseEfpComputation,
            45 => Self::LastTimestamp,
            46 => Self::Shortable,
            47 => Self::FundamentalRatios,
            48 => Self::RtVolume,
            49 => Self::Halted,
            50 => Self::BidYield,
            51 => Self::AskYield,
            52 => Self::LastYield,
            53 => Self::CustOptionComputation,
            54 => Self::TradeCount,
            55 => Self::TradeRate,
            56 => Self::VolumeRate,
            57 => Self::LastRthTrade,
            58 => Self::RtHistoricalVol,
            59 => Self::IbDividends,
            60 => Self::BondFactorMultiplier,
            61 => Self::RegulatoryImbalance,
            62 => Self::NewsTick,
            63 => Self::ShortTermVolume3Min,
            64 => Self::ShortTermVolume5Min,
            65 => Self::ShortTermVolume10Min,
            66 => Self::DelayedBid,
            67 => Self::DelayedAsk,
            68 => Self::DelayedLast,
            69 => Self::DelayedBidSize,
            70 => Self::DelayedAskSize,
            71 => Self::DelayedLastSize,
            72 => Self::DelayedHigh,
            73 => Self::DelayedLow,
            74 => Self::DelayedVolume,
            75 => Self::DelayedClose,
            76 => Self::DelayedOpen,
            77 => Self::RtTrdVolume,
            78 => Self::CreditmanMarkPrice,
            79 => Self::CreditmanSlowMarkPrice,
            80 => Self::DelayedBidOption,
            81 => Self::DelayedAskOption,
            82 => Self::DelayedLastOption,
            83 => Self::DelayedModelOption,
            84 => Self::LastExch,
            85 => Self::LastRegTime,
            86 => Self::FuturesOpenInterest,
            87 => Self::AvgOptVolume,
            88 => Self::DelayedLastTimestamp,
            89 => Self::ShortableShares,
            90 => Self::DelayedHalted,
            91 => Self::Reuters2MutualFunds,
            92 => Self::EtfNavClose,
            93 => Self::EtfNavPriorClose,
            94 => Self::EtfNavBid,
            95 => Self::EtfNavAsk,
            96 => Self::EtfNavLast,
            97 => Self::EtfFrozenNavLast,
            98 => Self::EtfNavHigh,
            99 => Self::EtfNavLow,
            100 => Self::SocialMarketAnalytics,
            101 => Self::EstimatedIpoMidpoint,
            102 => Self::FinalIpoLast,
            103 => Self::DelayedYieldBid,
            104 => Self::DelayedYieldAsk,
            _ => Self::Unknown,
        }
    }
}

impl From<&str> for TickType {
    fn from(value: &str) -> Self {
        match value {
            "bidSize" => Self::BidSize,
            "bidPrice" => Self::Bid,
            "askPrice" => Self::Ask,
            "askSize" => Self::AskSize,
            "lastPrice" => Self::Last,
            "lastSize" => Self::LastSize,
            "high" => Self::High,
            "low" => Self::Low,
            "volume" => Self::Volume,
            "close" => Self::Close,
            "bidOptComp" => Self::BidOption,
            "askOptComp" => Self::AskOption,
            "lastOptComp" => Self::LastOption,
            "modelOptComp" => Self::ModelOption,
            "open" => Self::Open,
            "13WeekLow" => Self::Low13Week,
            "13WeekHigh" => Self::High13Week,
            "26WeekLow" => Self::Low26Week,
            "26WeekHigh" => Self::High26Week,
            "52WeekLow" => Self::Low52Week,
            "52WeekHigh" => Self::High52Week,
            "AvgVolume" => Self::AvgVolume,
            "OpenInterest" => Self::OpenInterest,
            "OptionHistoricalVolatility" => Self::OptionHistoricalVol,
            "OptionImpliedVolatility" => Self::OptionImpliedVol,
            "OptionBidExchStr" => Self::OptionBidExch,
            "OptionAskExchStr" => Self::OptionAskExch,
            "OptionCallOpenInterest" => Self::OptionCallOpenInterest,
            "OptionPutOpenInterest" => Self::OptionPutOpenInterest,
            "OptionCallVolume" => Self::OptionCallVolume,
            "OptionPutVolume" => Self::OptionPutVolume,
            "IndexFuturePremium" => Self::IndexFuturePremium,
            "bidExch" => Self::BidExch,
            "askExch" => Self::AskExch,
            "auctionVolume" => Self::AuctionVolume,
            "auctionPrice" => Self::AuctionPrice,
            "auctionImbalance" => Self::AuctionImbalance,
            "markPrice" => Self::MarkPrice,
            "bidEFP" => Self::BidEfpComputation,
            "askEFP" => Self::AskEfpComputation,
            "lastEFP" => Self::LastEfpComputation,
            "openEFP" => Self::OpenEfpComputation,
            "highEFP" => Self::HighEfpComputation,
            "lowEFP" => Self::LowEfpComputation,
            "closeEFP" => Self::CloseEfpComputation,
            "lastTimestamp" => Self::LastTimestamp,
            "shortable" => Self::Shortable,
            "fundamentals" => Self::FundamentalRatios,
            "RTVolume" => Self::RtVolume,
            "halted" => Self::Halted,
            "bidYield" => Self::BidYield,
            "askYield" => Self::AskYield,
            "lastYield" => Self::LastYield,
            "custOptComp" => Self::CustOptionComputation,
            "trades" => Self::TradeCount,
            "trades/min" => Self::TradeRate,
            "volume/min" => Self::VolumeRate,
            "lastRTHTrade" => Self::LastRthTrade,
            "RTHistoricalVol" => Self::RtHistoricalVol,
            "IBDividends" => Self::IbDividends,
            "bondFactorMultiplier" => Self::BondFactorMultiplier,
            "regulatoryImbalance" => Self::RegulatoryImbalance,
            "newsTick" => Self::NewsTick,
            "shortTermVolume3Min" => Self::ShortTermVolume3Min,
            "shortTermVolume5Min" => Self::ShortTermVolume5Min,
            "shortTermVolume10Min" => Self::ShortTermVolume10Min,
            "delayedBid" => Self::DelayedBid,
            "delayedAsk" => Self::DelayedAsk,
            "delayedLast" => Self::DelayedLast,
            "delayedBidSize" => Self::DelayedBidSize,
            "delayedAskSize" => Self::DelayedAskSize,
            "delayedLastSize" => Self::DelayedLastSize,
            "delayedHigh" => Self::DelayedHigh,
            "delayedLow" => Self::DelayedLow,
            "delayedVolume" => Self::DelayedVolume,
            "delayedClose" => Self::DelayedClose,
            "delayedOpen" => Self::DelayedOpen,
            "rtTrdVolume" => Self::RtTrdVolume,
            "creditmanMarkPrice" => Self::CreditmanMarkPrice,
            "creditmanSlowMarkPrice" => Self::CreditmanSlowMarkPrice,
            "delayedBidOptComp" => Self::DelayedBidOption,
            "delayedAskOptComp" => Self::DelayedAskOption,
            "delayedLastOptComp" => Self::DelayedLastOption,
            "delayedModelOptComp" => Self::DelayedModelOption,
            "lastExchange" => Self::LastExch,
            "lastRegTime" => Self::LastRegTime,
            "futuresOpenInterest" => Self::FuturesOpenInterest,
            "avgOptVolume" => Self::AvgOptVolume,
            "delayedLastTimestamp" => Self::DelayedLastTimestamp,
            "shortableShares" => Self::ShortableShares,
            "delayedHalted" => Self::DelayedHalted,
            "reuters2MutualFunds" => Self::Reuters2MutualFunds,
            "etfNavClose" => Self::EtfNavClose,
            "etfNavPriorClose" => Self::EtfNavPriorClose,
            "etfNavBid" => Self::EtfNavBid,
            "etfNavAsk" => Self::EtfNavAsk,
            "etfNavLast" => Self::EtfNavLast,
            "etfFrozenNavLast" => Self::EtfFrozenNavLast,
            "etfNavHigh" => Self::EtfNavHigh,
            "etfNavLow" => Self::EtfNavLow,
            "socialMarketAnalytics" => Self::SocialMarketAnalytics,
            "estimatedIPOMidpoint" => Self::EstimatedIpoMidpoint,
            "finalIPOLast" => Self::FinalIpoLast,
            "delayedYieldBid" => Self::DelayedYieldBid,
            "delayedYieldAsk" => Self::DelayedYieldAsk,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for TickType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::BidSize => write!(f, "Bid Size"),
            Self::Bid => write!(f, "Bid"),
            Self::Ask => write!(f, "Ask"),
            Self::AskSize => write!(f, "Ask Size"),
            Self::Last => write!(f, "Last"),
            Self::LastSize => write!(f, "Last Size"),
            Self::High => write!(f, "High"),
            Self::Low => write!(f, "Low"),
            Self::Volume => write!(f, "Volume"),
            Self::Close => write!(f, "Close"),
            Self::BidOption => write!(f, "Bid Option"),
            Self::AskOption => write!(f, "Ask Option"),
            Self::LastOption => write!(f, "Last Option"),
            Self::ModelOption => write!(f, "Model Option"),
            Self::Open => write!(f, "Open"),
            Self::Low13Week => write!(f, "13 Week Low"),
            Self::High13Week => write!(f, "13 Week High"),
            Self::Low26Week => write!(f, "26 Week Low"),
            Self::High26Week => write!(f, "26 Week High"),
            Self::Low52Week => write!(f, "52 Week Low"),
            Self::High52Week => write!(f, "52 Week High"),
            Self::AvgVolume => write!(f, "Average Volume"),
            Self::OpenInterest => write!(f, "Open Interest"),
            Self::OptionHistoricalVol => write!(f, "Option Historical Volatility"),
            Self::OptionImpliedVol => write!(f, "Option Implied Volatility"),
            Self::OptionBidExch => write!(f, "Option Bid Exchange"),
            Self::OptionAskExch => write!(f, "Option Ask Exchange"),
            Self::OptionCallOpenInterest => write!(f, "Option Call Open Interest"),
            Self::OptionPutOpenInterest => write!(f, "Option Put Open Interest"),
            Self::OptionCallVolume => write!(f, "Option Call Volume"),
            Self::OptionPutVolume => write!(f, "Option Put Volume"),
            Self::IndexFuturePremium => write!(f, "Index Future Premium"),
            Self::BidExch => write!(f, "Bid Exchange"),
            Self::AskExch => write!(f, "Ask Exchange"),
            Self::AuctionVolume => write!(f, "Auction Volume"),
            Self::AuctionPrice => write!(f, "Auction Price"),
            Self::AuctionImbalance => write!(f, "Auction Imbalance"),
            Self::MarkPrice => write!(f, "Mark Price"),
            Self::BidEfpComputation => write!(f, "Bid EFP Computation"),
            Self::AskEfpComputation => write!(f, "Ask EFP Computation"),
            Self::LastEfpComputation => write!(f, "Last EFP Computation"),
            Self::OpenEfpComputation => write!(f, "Open EFP Computation"),
            Self::HighEfpComputation => write!(f, "High EFP Computation"),
            Self::LowEfpComputation => write!(f, "Low EFP Computation"),
            Self::CloseEfpComputation => write!(f, "Close EFP Computation"),
            Self::LastTimestamp => write!(f, "Last Timestamp"),
            Self::Shortable => write!(f, "Shortable"),
            Self::FundamentalRatios => write!(f, "Fundamental Ratios"),
            Self::RtVolume => write!(f, "RT Volume"),
            Self::Halted => write!(f, "Halted"),
            Self::BidYield => write!(f, "Bid Yield"),
            Self::AskYield => write!(f, "Ask Yield"),
            Self::LastYield => write!(f, "Last Yield"),
            Self::CustOptionComputation => write!(f, "Custom Option Computation"),
            Self::TradeCount => write!(f, "Trade Count"),
            Self::TradeRate => write!(f, "Trade Rate"),
            Self::VolumeRate => write!(f, "Volume Rate"),
            Self::LastRthTrade => write!(f, "Last RTH Trade"),
            Self::RtHistoricalVol => write!(f, "RT Historical Volatility"),
            Self::IbDividends => write!(f, "IB Dividends"),
            Self::BondFactorMultiplier => write!(f, "Bond Factor Multiplier"),
            Self::RegulatoryImbalance => write!(f, "Regulatory Imbalance"),
            Self::NewsTick => write!(f, "News Tick"),
            Self::ShortTermVolume3Min => write!(f, "Short Term Volume 3 Min"),
            Self::ShortTermVolume5Min => write!(f, "Short Term Volume 5 Min"),
            Self::ShortTermVolume10Min => write!(f, "Short Term Volume 10 Min"),
            Self::DelayedBid => write!(f, "Delayed Bid"),
            Self::DelayedAsk => write!(f, "Delayed Ask"),
            Self::DelayedLast => write!(f, "Delayed Last"),
            Self::DelayedBidSize => write!(f, "Delayed Bid Size"),
            Self::DelayedAskSize => write!(f, "Delayed Ask Size"),
            Self::DelayedLastSize => write!(f, "Delayed Last Size"),
            Self::DelayedHigh => write!(f, "Delayed High"),
            Self::DelayedLow => write!(f, "Delayed Low"),
            Self::DelayedVolume => write!(f, "Delayed Volume"),
            Self::DelayedClose => write!(f, "Delayed Close"),
            Self::DelayedOpen => write!(f, "Delayed Open"),
            Self::RtTrdVolume => write!(f, "RT Trade Volume"),
            Self::CreditmanMarkPrice => write!(f, "Creditman Mark Price"),
            Self::CreditmanSlowMarkPrice => write!(f, "Creditman Slow Mark Price"),
            Self::DelayedBidOption => write!(f, "Delayed Bid Option"),
            Self::DelayedAskOption => write!(f, "Delayed Ask Option"),
            Self::DelayedLastOption => write!(f, "Delayed Last Option"),
            Self::DelayedModelOption => write!(f, "Delayed Model Option"),
            Self::LastExch => write!(f, "Last Exchange"),
            Self::LastRegTime => write!(f, "Last Reg Time"),
            Self::FuturesOpenInterest => write!(f, "Futures Open Interest"),
            Self::AvgOptVolume => write!(f, "Average Option Volume"),
            Self::DelayedLastTimestamp => write!(f, "Delayed Last Timestamp"),
            Self::ShortableShares => write!(f, "Shortable Shares"),
            Self::DelayedHalted => write!(f, "Delayed Halted"),
            Self::Reuters2MutualFunds => write!(f, "Reuters2 Mutual Funds"),
            Self::EtfNavClose => write!(f, "ETF NAV Close"),
            Self::EtfNavPriorClose => write!(f, "ETF NAV Prior Close"),
            Self::EtfNavBid => write!(f, "ETF NAV Bid"),
            Self::EtfNavAsk => write!(f, "ETF NAV Ask"),
            Self::EtfNavLast => write!(f, "ETF NAV Last"),
            Self::EtfFrozenNavLast => write!(f, "ETF Frozen NAV Last"),
            Self::EtfNavHigh => write!(f, "ETF NAV High"),
            Self::EtfNavLow => write!(f, "ETF NAV Low"),
            Self::SocialMarketAnalytics => write!(f, "Social Market Analytics"),
            Self::EstimatedIpoMidpoint => write!(f, "Estimated IPO Midpoint"),
            Self::FinalIpoLast => write!(f, "Final IPO Last"),
            Self::DelayedYieldBid => write!(f, "Delayed Yield Bid"),
            Self::DelayedYieldAsk => write!(f, "Delayed Yield Ask"),
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use super::*;

    #[test]
    fn test_from_i32_all_values() {
        let test_cases = vec![
            (-1, TickType::Unknown),
            (0, TickType::BidSize),
            (1, TickType::Bid),
            (2, TickType::Ask),
            (3, TickType::AskSize),
            (4, TickType::Last),
            (5, TickType::LastSize),
            (6, TickType::High),
            (7, TickType::Low),
            (8, TickType::Volume),
            (9, TickType::Close),
            (10, TickType::BidOption),
            (11, TickType::AskOption),
            (12, TickType::LastOption),
            (13, TickType::ModelOption),
            (14, TickType::Open),
            (15, TickType::Low13Week),
            (16, TickType::High13Week),
            (17, TickType::Low26Week),
            (18, TickType::High26Week),
            (19, TickType::Low52Week),
            (20, TickType::High52Week),
            (21, TickType::AvgVolume),
            (22, TickType::OpenInterest),
            (23, TickType::OptionHistoricalVol),
            (24, TickType::OptionImpliedVol),
            (25, TickType::OptionBidExch),
            (26, TickType::OptionAskExch),
            (27, TickType::OptionCallOpenInterest),
            (28, TickType::OptionPutOpenInterest),
            (29, TickType::OptionCallVolume),
            (30, TickType::OptionPutVolume),
            (31, TickType::IndexFuturePremium),
            (32, TickType::BidExch),
            (33, TickType::AskExch),
            (34, TickType::AuctionVolume),
            (35, TickType::AuctionPrice),
            (36, TickType::AuctionImbalance),
            (37, TickType::MarkPrice),
            (38, TickType::BidEfpComputation),
            (39, TickType::AskEfpComputation),
            (40, TickType::LastEfpComputation),
            (41, TickType::OpenEfpComputation),
            (42, TickType::HighEfpComputation),
            (43, TickType::LowEfpComputation),
            (44, TickType::CloseEfpComputation),
            (45, TickType::LastTimestamp),
            (46, TickType::Shortable),
            (47, TickType::FundamentalRatios),
            (48, TickType::RtVolume),
            (49, TickType::Halted),
            (50, TickType::BidYield),
            (51, TickType::AskYield),
            (52, TickType::LastYield),
            (53, TickType::CustOptionComputation),
            (54, TickType::TradeCount),
            (55, TickType::TradeRate),
            (56, TickType::VolumeRate),
            (57, TickType::LastRthTrade),
            (58, TickType::RtHistoricalVol),
            (59, TickType::IbDividends),
            (60, TickType::BondFactorMultiplier),
            (61, TickType::RegulatoryImbalance),
            (62, TickType::NewsTick),
            (63, TickType::ShortTermVolume3Min),
            (64, TickType::ShortTermVolume5Min),
            (65, TickType::ShortTermVolume10Min),
            (66, TickType::DelayedBid),
            (67, TickType::DelayedAsk),
            (68, TickType::DelayedLast),
            (69, TickType::DelayedBidSize),
            (70, TickType::DelayedAskSize),
            (71, TickType::DelayedLastSize),
            (72, TickType::DelayedHigh),
            (73, TickType::DelayedLow),
            (74, TickType::DelayedVolume),
            (75, TickType::DelayedClose),
            (76, TickType::DelayedOpen),
            (77, TickType::RtTrdVolume),
            (78, TickType::CreditmanMarkPrice),
            (79, TickType::CreditmanSlowMarkPrice),
            (80, TickType::DelayedBidOption),
            (81, TickType::DelayedAskOption),
            (82, TickType::DelayedLastOption),
            (83, TickType::DelayedModelOption),
            (84, TickType::LastExch),
            (85, TickType::LastRegTime),
            (86, TickType::FuturesOpenInterest),
            (87, TickType::AvgOptVolume),
            (88, TickType::DelayedLastTimestamp),
            (89, TickType::ShortableShares),
            (90, TickType::DelayedHalted),
            (91, TickType::Reuters2MutualFunds),
            (92, TickType::EtfNavClose),
            (93, TickType::EtfNavPriorClose),
            (94, TickType::EtfNavBid),
            (95, TickType::EtfNavAsk),
            (96, TickType::EtfNavLast),
            (97, TickType::EtfFrozenNavLast),
            (98, TickType::EtfNavHigh),
            (99, TickType::EtfNavLow),
            (100, TickType::SocialMarketAnalytics),
            (101, TickType::EstimatedIpoMidpoint),
            (102, TickType::FinalIpoLast),
            (103, TickType::DelayedYieldBid),
            (104, TickType::DelayedYieldAsk),
            (105, TickType::Unknown),
            (-2, TickType::Unknown),
            (1000, TickType::Unknown),
        ];

        for (input, expected) in test_cases {
            assert_eq!(TickType::from(input), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_from_str_all_values() {
        let test_cases = vec![
            ("bidSize", TickType::BidSize),
            ("bidPrice", TickType::Bid),
            ("askPrice", TickType::Ask),
            ("askSize", TickType::AskSize),
            ("lastPrice", TickType::Last),
            ("lastSize", TickType::LastSize),
            ("high", TickType::High),
            ("low", TickType::Low),
            ("volume", TickType::Volume),
            ("close", TickType::Close),
            ("bidOptComp", TickType::BidOption),
            ("askOptComp", TickType::AskOption),
            ("lastOptComp", TickType::LastOption),
            ("modelOptComp", TickType::ModelOption),
            ("open", TickType::Open),
            ("13WeekLow", TickType::Low13Week),
            ("13WeekHigh", TickType::High13Week),
            ("26WeekLow", TickType::Low26Week),
            ("26WeekHigh", TickType::High26Week),
            ("52WeekLow", TickType::Low52Week),
            ("52WeekHigh", TickType::High52Week),
            ("AvgVolume", TickType::AvgVolume),
            ("OpenInterest", TickType::OpenInterest),
            ("OptionHistoricalVolatility", TickType::OptionHistoricalVol),
            ("OptionImpliedVolatility", TickType::OptionImpliedVol),
            ("OptionBidExchStr", TickType::OptionBidExch),
            ("OptionAskExchStr", TickType::OptionAskExch),
            ("OptionCallOpenInterest", TickType::OptionCallOpenInterest),
            ("OptionPutOpenInterest", TickType::OptionPutOpenInterest),
            ("OptionCallVolume", TickType::OptionCallVolume),
            ("OptionPutVolume", TickType::OptionPutVolume),
            ("IndexFuturePremium", TickType::IndexFuturePremium),
            ("bidExch", TickType::BidExch),
            ("askExch", TickType::AskExch),
            ("auctionVolume", TickType::AuctionVolume),
            ("auctionPrice", TickType::AuctionPrice),
            ("auctionImbalance", TickType::AuctionImbalance),
            ("markPrice", TickType::MarkPrice),
            ("bidEFP", TickType::BidEfpComputation),
            ("askEFP", TickType::AskEfpComputation),
            ("lastEFP", TickType::LastEfpComputation),
            ("openEFP", TickType::OpenEfpComputation),
            ("highEFP", TickType::HighEfpComputation),
            ("lowEFP", TickType::LowEfpComputation),
            ("closeEFP", TickType::CloseEfpComputation),
            ("lastTimestamp", TickType::LastTimestamp),
            ("shortable", TickType::Shortable),
            ("fundamentals", TickType::FundamentalRatios),
            ("RTVolume", TickType::RtVolume),
            ("halted", TickType::Halted),
            ("bidYield", TickType::BidYield),
            ("askYield", TickType::AskYield),
            ("lastYield", TickType::LastYield),
            ("custOptComp", TickType::CustOptionComputation),
            ("trades", TickType::TradeCount),
            ("trades/min", TickType::TradeRate),
            ("volume/min", TickType::VolumeRate),
            ("lastRTHTrade", TickType::LastRthTrade),
            ("RTHistoricalVol", TickType::RtHistoricalVol),
            ("IBDividends", TickType::IbDividends),
            ("bondFactorMultiplier", TickType::BondFactorMultiplier),
            ("regulatoryImbalance", TickType::RegulatoryImbalance),
            ("newsTick", TickType::NewsTick),
            ("shortTermVolume3Min", TickType::ShortTermVolume3Min),
            ("shortTermVolume5Min", TickType::ShortTermVolume5Min),
            ("shortTermVolume10Min", TickType::ShortTermVolume10Min),
            ("delayedBid", TickType::DelayedBid),
            ("delayedAsk", TickType::DelayedAsk),
            ("delayedLast", TickType::DelayedLast),
            ("delayedBidSize", TickType::DelayedBidSize),
            ("delayedAskSize", TickType::DelayedAskSize),
            ("delayedLastSize", TickType::DelayedLastSize),
            ("delayedHigh", TickType::DelayedHigh),
            ("delayedLow", TickType::DelayedLow),
            ("delayedVolume", TickType::DelayedVolume),
            ("delayedClose", TickType::DelayedClose),
            ("delayedOpen", TickType::DelayedOpen),
            ("rtTrdVolume", TickType::RtTrdVolume),
            ("creditmanMarkPrice", TickType::CreditmanMarkPrice),
            ("creditmanSlowMarkPrice", TickType::CreditmanSlowMarkPrice),
            ("delayedBidOptComp", TickType::DelayedBidOption),
            ("delayedAskOptComp", TickType::DelayedAskOption),
            ("delayedLastOptComp", TickType::DelayedLastOption),
            ("delayedModelOptComp", TickType::DelayedModelOption),
            ("lastExchange", TickType::LastExch),
            ("lastRegTime", TickType::LastRegTime),
            ("futuresOpenInterest", TickType::FuturesOpenInterest),
            ("avgOptVolume", TickType::AvgOptVolume),
            ("delayedLastTimestamp", TickType::DelayedLastTimestamp),
            ("shortableShares", TickType::ShortableShares),
            ("delayedHalted", TickType::DelayedHalted),
            ("reuters2MutualFunds", TickType::Reuters2MutualFunds),
            ("etfNavClose", TickType::EtfNavClose),
            ("etfNavPriorClose", TickType::EtfNavPriorClose),
            ("etfNavBid", TickType::EtfNavBid),
            ("etfNavAsk", TickType::EtfNavAsk),
            ("etfNavLast", TickType::EtfNavLast),
            ("etfFrozenNavLast", TickType::EtfFrozenNavLast),
            ("etfNavHigh", TickType::EtfNavHigh),
            ("etfNavLow", TickType::EtfNavLow),
            ("socialMarketAnalytics", TickType::SocialMarketAnalytics),
            ("estimatedIPOMidpoint", TickType::EstimatedIpoMidpoint),
            ("finalIPOLast", TickType::FinalIpoLast),
            ("delayedYieldBid", TickType::DelayedYieldBid),
            ("delayedYieldAsk", TickType::DelayedYieldAsk),
            ("nonexistent", TickType::Unknown),
            ("", TickType::Unknown),
            ("  ", TickType::Unknown),
            ("BIDSIZE", TickType::Unknown),
            ("BidSize", TickType::Unknown),
        ];

        for (input, expected) in test_cases {
            assert_eq!(TickType::from(input), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_default() {
        assert_eq!(TickType::default(), TickType::Unknown);
    }

    #[test]
    fn test_debug_output() {
        assert_eq!(format!("{:?}", TickType::Bid), "Bid");
        assert_eq!(format!("{:?}", TickType::AskSize), "AskSize");
        assert_eq!(format!("{:?}", TickType::DelayedYieldAsk), "DelayedYieldAsk");
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(TickType::Last, TickType::Last);
        assert_ne!(TickType::High, TickType::Low);
    }

    #[test]
    fn test_edge_cases() {
        // Test the lowest and highest defined values
        assert_eq!(TickType::from(0), TickType::BidSize);
        assert_eq!(TickType::from(104), TickType::DelayedYieldAsk);

        // Test values just outside the defined range
        assert_eq!(TickType::from(-2), TickType::Unknown);
        assert_eq!(TickType::from(105), TickType::Unknown);

        // Test with empty string and whitespace
        assert_eq!(TickType::from(""), TickType::Unknown);
        assert_eq!(TickType::from("  "), TickType::Unknown);
    }

    #[test]
    fn test_case_sensitivity() {
        assert_eq!(TickType::from("BIDSIZE"), TickType::Unknown);
        assert_eq!(TickType::from("bidSize"), TickType::BidSize);
        assert_eq!(TickType::from("BidSize"), TickType::Unknown);
    }

    #[test]
    fn test_display_output() {
        assert_eq!(format!("{}", TickType::Bid), "Bid");
        assert_eq!(format!("{}", TickType::AskSize), "Ask Size");
        assert_eq!(format!("{}", TickType::Last), "Last");
        assert_eq!(format!("{}", TickType::Volume), "Volume");
        assert_eq!(format!("{}", TickType::DelayedYieldAsk), "Delayed Yield Ask");
        assert_eq!(format!("{}", TickType::Unknown), "Unknown");
        assert_eq!(format!("{}", TickType::OptionImpliedVol), "Option Implied Volatility");
        assert_eq!(format!("{}", TickType::ShortableShares), "Shortable Shares");
    }
}
