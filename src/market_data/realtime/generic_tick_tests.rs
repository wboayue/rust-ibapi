use super::*;

/// Regression guard: every constant must match the numeric ID listed at
/// <https://www.interactivebrokers.com/docs/tws-api/doc/market-data-live/available-tick-types/introduction>.
///
/// Table is the source-of-truth column. If TWS adds a new generic tick request
/// ID, add a row here and a constant on `GenericTick` so the constants stay in
/// sync with the IB docs page.
#[test]
fn constants_match_documented_numeric_ids() {
    let cases = [
        (GenericTick::OPTION_VOLUME, 100),
        (GenericTick::OPTION_OPEN_INTEREST, 101),
        (GenericTick::OPTION_HISTORICAL_VOLATILITY, 104),
        (GenericTick::AVERAGE_OPTION_VOLUME, 105),
        (GenericTick::OPTION_IMPLIED_VOLATILITY, 106),
        (GenericTick::INDEX_FUTURE_PREMIUM, 162),
        (GenericTick::MISC_STATS, 165),
        (GenericTick::AUCTION_VALUES, 225),
        (GenericTick::MARK_PRICE, 232),
        (GenericTick::RT_VOLUME, 233),
        (GenericTick::SHORTABLE, 236),
        (GenericTick::NEWS, 292),
        (GenericTick::TRADE_COUNT, 293),
        (GenericTick::TRADE_RATE, 294),
        (GenericTick::VOLUME_RATE, 295),
        (GenericTick::LAST_RTH_TRADE, 318),
        (GenericTick::RT_TRADE_VOLUME, 375),
        (GenericTick::RT_HISTORICAL_VOLATILITY, 411),
        (GenericTick::IB_DIVIDENDS, 456),
        (GenericTick::BOND_FACTOR_MULTIPLIER, 460),
        (GenericTick::ETF_NAV_LAST, 577),
        (GenericTick::IPO_PRICES, 586),
        (GenericTick::FUTURES_OPEN_INTEREST, 588),
        (GenericTick::SHORT_TERM_VOLUME, 595),
        (GenericTick::ETF_NAV_HIGH_LOW, 614),
        (GenericTick::CREDITMAN_SLOW_MARK_PRICE, 619),
        (GenericTick::ETF_FROZEN_NAV_LAST, 623),
        (GenericTick::COMBO_IMPLIED_BID_ASK, 760),
        (GenericTick::ODD_LOT, 787),
    ];

    for (constant, expected) in cases {
        assert_eq!(constant.id(), expected);
        assert_eq!(constant, GenericTick::from(expected));
        assert_eq!(constant.to_string(), expected.to_string(), "sent as the number");
    }
}

#[test]
fn converts_from_a_number_and_from_a_reference() {
    let tick = GenericTick::from(258);
    assert_eq!(tick.id(), 258);
    assert_eq!(GenericTick::from(&tick), tick);
}
