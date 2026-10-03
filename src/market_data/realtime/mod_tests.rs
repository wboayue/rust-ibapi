use super::*;

#[test]
fn test_what_to_show_display() {
    assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
    assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
    assert_eq!(WhatToShow::Bid.to_string(), "BID");
    assert_eq!(WhatToShow::Ask.to_string(), "ASK");
    assert_eq!(WhatToShow::AggTrades.to_string(), "AGGTRADES");
}

/// Only 317 becomes `Reset`; other notices on a depth stream, including the
/// other data advisories, stay `Notice` (#899).
#[test]
fn test_market_depths_data_from_notice() {
    let notice = |code| Notice::synthesized(code, String::new());

    assert_eq!(
        <MarketDepths as StreamDecoder<MarketDepths>>::data_from_notice(&notice(317)),
        Some(MarketDepths::Reset)
    );
    for code in [316, 2104, 2188, 10167] {
        assert_eq!(
            <MarketDepths as StreamDecoder<MarketDepths>>::data_from_notice(&notice(code)),
            None,
            "code {code}"
        );
    }
}
