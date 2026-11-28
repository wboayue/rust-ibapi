use crate::market_data::{TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint};
use crate::proto;
use crate::proto::decoders::{parse_decimal_or_zero, parse_optional_decimal, ts};
use crate::Error;

pub(in crate::market_data) fn decode_tick_last(t: proto::HistoricalTickLast) -> Result<TickLast, Error> {
    let attr = t.tick_attrib_last.as_ref();
    Ok(TickLast {
        time: ts(t.time.unwrap_or_default()),
        price: t.price.unwrap_or_default(),
        size: parse_decimal_or_zero(t.size.as_deref())?,
        attributes: TickAttributeLast {
            past_limit: attr.and_then(|a| a.past_limit).unwrap_or_default(),
            unreported: attr.and_then(|a| a.unreported).unwrap_or_default(),
        },
        exchange: t.exchange.unwrap_or_default(),
        special_conditions: t.special_conditions.unwrap_or_default(),
    })
}

pub(in crate::market_data) fn decode_tick_bid_ask(t: proto::HistoricalTickBidAsk) -> Result<TickBidAsk, Error> {
    let attr = t.tick_attrib_bid_ask.as_ref();
    Ok(TickBidAsk {
        time: ts(t.time.unwrap_or_default()),
        price_bid: t.price_bid.unwrap_or_default(),
        price_ask: t.price_ask.unwrap_or_default(),
        size_bid: parse_decimal_or_zero(t.size_bid.as_deref())?,
        size_ask: parse_decimal_or_zero(t.size_ask.as_deref())?,
        attributes: TickAttributeBidAsk {
            bid_past_low: attr.and_then(|a| a.bid_past_low).unwrap_or_default(),
            ask_past_high: attr.and_then(|a| a.ask_past_high).unwrap_or_default(),
        },
    })
}

/// Infallible: the wire `size` is not decoded (see [`TickMidpoint`]).
pub(in crate::market_data) fn decode_tick_midpoint(t: proto::HistoricalTick) -> TickMidpoint {
    TickMidpoint {
        time: ts(t.time.unwrap_or_default()),
        price: t.price.unwrap_or_default(),
    }
}

/// A bar's trade fields: volume, WAP and trade count, each `None` when the bar has no trade data.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(in crate::market_data) struct BarTradeFields {
    pub(in crate::market_data) volume: Option<f64>,
    pub(in crate::market_data) wap: Option<f64>,
    pub(in crate::market_data) count: Option<i32>,
}

/// TWS's "no trade data" marker on a bar. Historical bars send it for volume, WAP and count on
/// a bar type without trades (MIDPOINT, BID, ASK, BID_ASK); realtime bars send it for count and
/// send it or leave it out for volume and WAP. A contract with a price magnifier gets the WAP as
/// `-1` times the magnifier (`docs/wire-facts/market-data.md#bar-trade-fields-volume-wap-count`).
const NO_TRADE_DATA: i32 = -1;

/// Decodes the trade fields shared by realtime and historical bars, by value rather than by
/// `WhatToShow`: ADJUSTED_LAST and AGGTRADES carry real trade values too.
///
/// Volume and count are `None` when absent or `-1`. WAP is a price, so it is not tested for
/// `-1`; it is `None` whenever volume is. A TRADES bar in which nothing traded keeps
/// `Some(0.0)` / `Some(0)`.
pub(in crate::market_data) fn decode_bar_trade_fields(volume: Option<&str>, wap: Option<&str>, count: Option<i32>) -> Result<BarTradeFields, Error> {
    // Exact compare: TWS sends the literal `"-1"`, which parses to exactly -1.0.
    let volume = parse_optional_decimal(volume)?.filter(|&v| v != f64::from(NO_TRADE_DATA));
    let wap = match volume {
        Some(_) => parse_optional_decimal(wap)?,
        None => None,
    };
    let count = count.filter(|&c| c != NO_TRADE_DATA);
    Ok(BarTradeFields { volume, wap, count })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
