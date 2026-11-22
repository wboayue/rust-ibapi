use crate::market_data::{TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint};
use crate::proto;
use crate::proto::decoders::{parse_decimal_or_zero, ts};
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

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
