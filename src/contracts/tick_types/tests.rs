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
