//! Inline proto-message helpers for sync/async/decoders test files. These
//! mirror the on-wire shape produced by TWS for the realtime market-data
//! family (TickByTick / RealTimeBars / MarketData / MarketDepth) and pair with
//! `crate::common::test_utils::helpers::proto_response`.

use crate::proto;

pub(crate) fn encode<M: prost::Message>(msg: &M) -> Vec<u8> {
    let mut bytes = Vec::new();
    msg.encode(&mut bytes).expect("proto encode");
    bytes
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn proto_realtime_bar(time: i64, open: f64, high: f64, low: f64, close: f64, volume: f64, wap: f64, count: i32) -> proto::RealTimeBarTick {
    proto::RealTimeBarTick {
        req_id: Some(9001),
        time: Some(time),
        open: Some(open),
        high: Some(high),
        low: Some(low),
        close: Some(close),
        volume: Some(volume.to_string()),
        wap: Some(wap.to_string()),
        count: Some(count),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn proto_trade(
    tick_type: i32,
    time: i64,
    price: f64,
    size: f64,
    mask: i32,
    exchange: &str,
    special_conditions: &str,
) -> proto::TickByTickData {
    proto::TickByTickData {
        req_id: Some(9001),
        tick_type: Some(tick_type),
        tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickLast(proto::HistoricalTickLast {
            time: Some(time),
            tick_attrib_last: Some(proto::TickAttribLast {
                past_limit: Some(mask & 0x1 != 0),
                unreported: Some(mask & 0x2 != 0),
            }),
            price: Some(price),
            size: Some(size.to_string()),
            exchange: Some(exchange.to_string()),
            special_conditions: Some(special_conditions.to_string()),
        })),
    }
}

pub(crate) fn proto_bid_ask(time: i64, bid_price: f64, ask_price: f64, bid_size: f64, ask_size: f64, mask: i32) -> proto::TickByTickData {
    proto::TickByTickData {
        req_id: Some(9001),
        tick_type: Some(3),
        tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickBidAsk(proto::HistoricalTickBidAsk {
            time: Some(time),
            tick_attrib_bid_ask: Some(proto::TickAttribBidAsk {
                bid_past_low: Some(mask & 0x1 != 0),
                ask_past_high: Some(mask & 0x2 != 0),
            }),
            price_bid: Some(bid_price),
            price_ask: Some(ask_price),
            size_bid: Some(bid_size.to_string()),
            size_ask: Some(ask_size.to_string()),
        })),
    }
}

pub(crate) fn proto_mid_point(time: i64, mid_point: f64) -> proto::TickByTickData {
    proto::TickByTickData {
        req_id: Some(9001),
        tick_type: Some(4),
        tick: Some(proto::tick_by_tick_data::Tick::HistoricalTickMidPoint(proto::HistoricalTick {
            time: Some(time),
            price: Some(mid_point),
            size: Some("0".into()),
        })),
    }
}

pub(crate) fn proto_market_depth(position: i32, operation: i32, side: i32, price: f64, size: f64) -> proto::MarketDepth {
    proto::MarketDepth {
        req_id: Some(9001),
        market_depth_data: Some(proto::MarketDepthData {
            position: Some(position),
            operation: Some(operation),
            side: Some(side),
            price: Some(price),
            size: Some(size.to_string()),
            market_maker: None,
            is_smart_depth: None,
        }),
    }
}

pub(crate) fn proto_tick_price(tick_type: i32, price: f64, size: Option<f64>, attr_mask: i32) -> proto::TickPrice {
    proto::TickPrice {
        req_id: Some(9001),
        tick_type: Some(tick_type),
        price: Some(price),
        size: size.map(|s| s.to_string()),
        attr_mask: Some(attr_mask),
    }
}

pub(crate) fn proto_tick_size(tick_type: i32, size: f64) -> proto::TickSize {
    proto::TickSize {
        req_id: Some(9001),
        tick_type: Some(tick_type),
        size: Some(size.to_string()),
    }
}

pub(crate) fn proto_tick_string(tick_type: i32, value: &str) -> proto::TickString {
    proto::TickString {
        req_id: Some(9001),
        tick_type: Some(tick_type),
        value: Some(value.to_string()),
    }
}

pub(crate) fn proto_tick_generic(tick_type: i32, value: f64) -> proto::TickGeneric {
    proto::TickGeneric {
        req_id: Some(9001),
        tick_type: Some(tick_type),
        value: Some(value),
    }
}
