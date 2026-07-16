use super::*;

/// Regression guard: every constant must match the numeric ID listed at
/// <https://interactivebrokers.github.io/tws-api/tick_types.html>.
///
/// Table is the source-of-truth column. If TWS adds a new generic tick request
/// ID, add a row here and a `pub const` above so the constants module stays in
/// sync with the IB docs page.
#[test]
fn constants_match_documented_numeric_ids() {
    let cases = [
        (OPTION_VOLUME, "100"),
        (OPTION_OPEN_INTEREST, "101"),
        (OPTION_HISTORICAL_VOLATILITY, "104"),
        (AVERAGE_OPTION_VOLUME, "105"),
        (OPTION_IMPLIED_VOLATILITY, "106"),
        (INDEX_FUTURE_PREMIUM, "162"),
        (MISC_STATS, "165"),
        (AUCTION_VALUES, "225"),
        (MARK_PRICE, "232"),
        (RT_VOLUME, "233"),
        (SHORTABLE, "236"),
        (NEWS, "292"),
        (TRADE_COUNT, "293"),
        (TRADE_RATE, "294"),
        (VOLUME_RATE, "295"),
        (LAST_RTH_TRADE, "318"),
        (RT_TRADE_VOLUME, "375"),
        (RT_HISTORICAL_VOLATILITY, "411"),
        (IB_DIVIDENDS, "456"),
        (BOND_FACTOR_MULTIPLIER, "460"),
        (ETF_NAV_BID, "576"),
        (ETF_NAV_LAST, "577"),
        (ETF_NAV_FROZEN_LAST, "578"),
        (IPO_PRICES, "586"),
        (FUTURES_OPEN_INTEREST, "588"),
        (SHORT_TERM_VOLUME, "595"),
        (ETF_NAV_HIGH_LOW, "614"),
        (CREDITMAN_SLOW_MARK_PRICE, "619"),
        (ODD_LOT, "787"),
    ];

    for (constant, expected) in cases {
        assert_eq!(constant, expected, "constant {constant} should equal {expected}");
    }
}
