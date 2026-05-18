use prost::Message;

use crate::contracts::OptionComputation;
use crate::messages::ResponseMessage;
use crate::proto::decoders::{optional_f64, optional_string_f64, parse_f64, ts};
use crate::server_versions;
use crate::Error;

use crate::market_data::realtime::{
    Bar, BidAsk, BidAskAttribute, DepthMarketDataDescription, MarketDepth, MarketDepthL2, MidPoint, TickAttribute, TickEFP, TickGeneric, TickPrice,
    TickPriceSize, TickRequestParameters, TickSize, TickString, TickType, TickTypes, Trade, TradeAttribute,
};
use crate::market_data::MarketDataType;

pub(crate) fn decode_realtime_bar(message: &mut ResponseMessage) -> Result<Bar, Error> {
    decode_realtime_bar_proto(message.require_proto()?)
}

pub(crate) fn decode_trade_tick(message: &mut ResponseMessage) -> Result<Trade, Error> {
    decode_trade_tick_proto(message.require_proto()?)
}

pub(crate) fn decode_bid_ask_tick(message: &mut ResponseMessage) -> Result<BidAsk, Error> {
    decode_bid_ask_tick_proto(message.require_proto()?)
}

pub(crate) fn decode_mid_point_tick(message: &mut ResponseMessage) -> Result<MidPoint, Error> {
    decode_mid_point_tick_proto(message.require_proto()?)
}

pub(crate) fn decode_market_depth(message: &mut ResponseMessage) -> Result<MarketDepth, Error> {
    decode_market_depth_proto(message.require_proto()?)
}

pub(crate) fn decode_market_depth_l2(message: &mut ResponseMessage) -> Result<MarketDepthL2, Error> {
    decode_market_depth_l2_proto(message.require_proto()?)
}

// Stays dual-format: outgoing gate `PROTOBUF_REST_MESSAGES_3` (213) > floor 210.
pub(crate) fn decode_market_depth_exchanges(server_version: i32, message: &mut ResponseMessage) -> Result<Vec<DepthMarketDataDescription>, Error> {
    message.decode_proto_or_text(decode_market_depth_exchanges_proto, |msg| {
        msg.skip(); // message type
        let count = msg.next_int()?;
        let mut descriptions = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let description = if server_version >= server_versions::SERVICE_DATA_TYPE {
                DepthMarketDataDescription {
                    exchange_name: msg.next_string()?,
                    security_type: msg.next_string()?,
                    listing_exchange: msg.next_string()?,
                    service_data_type: msg.next_string()?,
                    aggregated_group: Some(msg.next_string()?),
                }
            } else {
                DepthMarketDataDescription {
                    exchange_name: msg.next_string()?,
                    security_type: msg.next_string()?,
                    listing_exchange: "".into(),
                    service_data_type: if msg.next_bool()? { "Deep2".into() } else { "Deep".into() },
                    aggregated_group: None,
                }
            };
            descriptions.push(description);
        }
        Ok(descriptions)
    })
}

pub(crate) fn decode_tick_price(message: &mut ResponseMessage) -> Result<TickTypes, Error> {
    decode_tick_price_proto(message.require_proto()?)
}

pub(crate) fn decode_tick_size(message: &mut ResponseMessage) -> Result<TickSize, Error> {
    decode_tick_size_proto(message.require_proto()?)
}

pub(crate) fn decode_tick_string(message: &mut ResponseMessage) -> Result<TickString, Error> {
    decode_tick_string_proto(message.require_proto()?)
}

// Stays text-only: TWS has no protobuf encoder for TickEFP.
pub(crate) fn decode_tick_efp(message: &mut ResponseMessage) -> Result<TickEFP, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    Ok(TickEFP {
        tick_type: TickType::from(message.next_int()?),
        basis_points: message.next_double()?,
        formatted_basis_points: message.next_string()?,
        implied_futures_price: message.next_double()?,
        hold_days: message.next_int()?,
        future_last_trade_date: message.next_string()?,
        dividend_impact: message.next_double()?,
        dividends_to_last_trade_date: message.next_double()?,
    })
}

pub(crate) fn decode_tick_generic(message: &mut ResponseMessage) -> Result<TickGeneric, Error> {
    decode_tick_generic_proto(message.require_proto()?)
}

pub(crate) fn decode_tick_option_computation(message: &mut ResponseMessage) -> Result<OptionComputation, Error> {
    decode_tick_option_computation_proto(message.require_proto()?)
}

pub(crate) fn decode_tick_request_parameters(message: &mut ResponseMessage) -> Result<TickRequestParameters, Error> {
    decode_tick_request_parameters_proto(message.require_proto()?)
}

pub(crate) fn decode_market_data_type(message: &mut ResponseMessage) -> Result<MarketDataType, Error> {
    decode_market_data_type_proto(message.require_proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_realtime_bar_proto(bytes: &[u8]) -> Result<Bar, Error> {
    let msg = crate::proto::RealTimeBarTick::decode(bytes)?;
    Ok(Bar {
        date: ts(msg.time.unwrap_or_default()),
        open: msg.open.unwrap_or_default(),
        high: msg.high.unwrap_or_default(),
        low: msg.low.unwrap_or_default(),
        close: msg.close.unwrap_or_default(),
        volume: parse_f64(&msg.volume),
        wap: parse_f64(&msg.wap),
        count: msg.count.unwrap_or_default(),
    })
}

pub(crate) fn decode_trade_tick_proto(bytes: &[u8]) -> Result<Trade, Error> {
    let msg = crate::proto::TickByTickData::decode(bytes)?;
    let tick_type = msg.tick_type.unwrap_or_default();
    if !(tick_type == 1 || tick_type == 2) {
        return Err(Error::parse_field(tick_type.to_string(), "Unexpected tick_type"));
    }
    let Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickLast(t)) = msg.tick else {
        return Err(Error::parse_proto("tick", "missing HistoricalTickLast in TickByTickData"));
    };
    let attr = t.tick_attrib_last.as_ref();
    Ok(Trade {
        tick_type: tick_type.to_string(),
        time: ts(t.time.unwrap_or_default()),
        price: t.price.unwrap_or_default(),
        size: parse_f64(&t.size),
        trade_attribute: TradeAttribute {
            past_limit: attr.and_then(|a| a.past_limit).unwrap_or_default(),
            unreported: attr.and_then(|a| a.unreported).unwrap_or_default(),
        },
        exchange: t.exchange.unwrap_or_default(),
        special_conditions: t.special_conditions.unwrap_or_default(),
    })
}

pub(crate) fn decode_bid_ask_tick_proto(bytes: &[u8]) -> Result<BidAsk, Error> {
    let msg = crate::proto::TickByTickData::decode(bytes)?;
    let tick_type = msg.tick_type.unwrap_or_default();
    if tick_type != 3 {
        return Err(Error::parse_field(tick_type.to_string(), "Unexpected tick_type"));
    }
    let Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickBidAsk(t)) = msg.tick else {
        return Err(Error::parse_proto("tick", "missing HistoricalTickBidAsk in TickByTickData"));
    };
    let attr = t.tick_attrib_bid_ask.as_ref();
    Ok(BidAsk {
        time: ts(t.time.unwrap_or_default()),
        bid_price: t.price_bid.unwrap_or_default(),
        ask_price: t.price_ask.unwrap_or_default(),
        bid_size: parse_f64(&t.size_bid),
        ask_size: parse_f64(&t.size_ask),
        bid_ask_attribute: BidAskAttribute {
            bid_past_low: attr.and_then(|a| a.bid_past_low).unwrap_or_default(),
            ask_past_high: attr.and_then(|a| a.ask_past_high).unwrap_or_default(),
        },
    })
}

pub(crate) fn decode_mid_point_tick_proto(bytes: &[u8]) -> Result<MidPoint, Error> {
    let msg = crate::proto::TickByTickData::decode(bytes)?;
    let tick_type = msg.tick_type.unwrap_or_default();
    if tick_type != 4 {
        return Err(Error::parse_field(tick_type.to_string(), "Unexpected tick_type"));
    }
    let Some(crate::proto::tick_by_tick_data::Tick::HistoricalTickMidPoint(t)) = msg.tick else {
        return Err(Error::parse_proto("tick", "missing HistoricalTickMidPoint in TickByTickData"));
    };
    Ok(MidPoint {
        time: ts(t.time.unwrap_or_default()),
        mid_point: t.price.unwrap_or_default(),
    })
}

pub(crate) fn decode_market_depth_exchanges_proto(bytes: &[u8]) -> Result<Vec<DepthMarketDataDescription>, Error> {
    let p = crate::proto::MarketDepthExchanges::decode(bytes)?;
    Ok(p.depth_market_data_descriptions
        .into_iter()
        .map(|d| DepthMarketDataDescription {
            exchange_name: d.exchange.unwrap_or_default(),
            security_type: d.sec_type.unwrap_or_default(),
            listing_exchange: d.listing_exch.unwrap_or_default(),
            service_data_type: d.service_data_type.unwrap_or_default(),
            aggregated_group: d.agg_group.map(|g| g.to_string()),
        })
        .collect())
}

pub(crate) fn decode_market_data_type_proto(bytes: &[u8]) -> Result<MarketDataType, Error> {
    let msg = crate::proto::MarketDataType::decode(bytes)?;
    Ok(MarketDataType::from(msg.market_data_type.unwrap_or_default()))
}

pub(crate) fn decode_tick_request_parameters_proto(bytes: &[u8]) -> Result<TickRequestParameters, Error> {
    let msg = crate::proto::TickReqParams::decode(bytes)?;
    Ok(TickRequestParameters {
        min_tick: parse_f64(&msg.min_tick),
        bbo_exchange: msg.bbo_exchange.unwrap_or_default(),
        snapshot_permissions: msg.snapshot_permissions.unwrap_or_default(),
    })
}

pub(crate) fn decode_tick_price_proto(bytes: &[u8]) -> Result<TickTypes, Error> {
    let msg = crate::proto::TickPrice::decode(bytes)?;

    let tick_type = TickType::from(msg.tick_type.unwrap_or_default());
    let price = msg.price.unwrap_or_default();
    let size = optional_string_f64(&msg.size);
    let attr_mask = msg.attr_mask.unwrap_or_default();

    let attributes = TickAttribute {
        can_auto_execute: attr_mask & 0x1 != 0,
        past_limit: attr_mask & 0x2 != 0,
        pre_open: attr_mask & 0x4 != 0,
    };

    let size_tick_type = match tick_type {
        TickType::Bid => TickType::BidSize,
        TickType::Ask => TickType::AskSize,
        TickType::Last => TickType::LastSize,
        TickType::DelayedBid => TickType::DelayedBidSize,
        TickType::DelayedAsk => TickType::DelayedAskSize,
        TickType::DelayedLast => TickType::DelayedLastSize,
        _ => TickType::Unknown,
    };

    match (size_tick_type, size) {
        (TickType::Unknown, _) | (_, None) => Ok(TickTypes::Price(TickPrice {
            tick_type,
            price,
            attributes,
        })),
        (size_tick_type, Some(size)) => Ok(TickTypes::PriceSize(TickPriceSize {
            price_tick_type: tick_type,
            price,
            attributes,
            size_tick_type,
            size,
        })),
    }
}

pub(crate) fn decode_tick_size_proto(bytes: &[u8]) -> Result<TickSize, Error> {
    let msg = crate::proto::TickSize::decode(bytes)?;

    Ok(TickSize {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        size: parse_f64(&msg.size),
    })
}

pub(crate) fn decode_tick_string_proto(bytes: &[u8]) -> Result<TickString, Error> {
    let msg = crate::proto::TickString::decode(bytes)?;

    Ok(TickString {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        value: msg.value.unwrap_or_default(),
    })
}

pub(crate) fn decode_tick_generic_proto(bytes: &[u8]) -> Result<TickGeneric, Error> {
    let msg = crate::proto::TickGeneric::decode(bytes)?;

    Ok(TickGeneric {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        value: msg.value.unwrap_or_default(),
    })
}

pub(crate) fn decode_tick_option_computation_proto(bytes: &[u8]) -> Result<OptionComputation, Error> {
    let msg = crate::proto::TickOptionComputation::decode(bytes)?;

    Ok(OptionComputation {
        field: TickType::from(msg.tick_type.unwrap_or_default()),
        tick_attribute: msg.tick_attrib,
        implied_volatility: optional_f64(msg.implied_vol),
        delta: optional_f64(msg.delta),
        option_price: optional_f64(msg.opt_price),
        present_value_dividend: optional_f64(msg.pv_dividend),
        gamma: optional_f64(msg.gamma),
        vega: optional_f64(msg.vega),
        theta: optional_f64(msg.theta),
        underlying_price: optional_f64(msg.und_price),
    })
}

pub(crate) fn decode_market_depth_proto(bytes: &[u8]) -> Result<MarketDepth, Error> {
    let msg = crate::proto::MarketDepth::decode(bytes)?;

    let data = msg
        .market_depth_data
        .ok_or_else(|| Error::UnexpectedResponse("missing market_depth_data".into()))?;

    Ok(MarketDepth {
        position: data.position.unwrap_or_default(),
        operation: data.operation.unwrap_or_default(),
        side: data.side.unwrap_or_default(),
        price: data.price.unwrap_or_default(),
        size: parse_f64(&data.size),
    })
}

pub(crate) fn decode_market_depth_l2_proto(bytes: &[u8]) -> Result<MarketDepthL2, Error> {
    let msg = crate::proto::MarketDepthL2::decode(bytes)?;

    let data = msg
        .market_depth_data
        .ok_or_else(|| Error::UnexpectedResponse("missing market_depth_data".into()))?;

    Ok(MarketDepthL2 {
        position: data.position.unwrap_or_default(),
        market_maker: data.market_maker.unwrap_or_default(),
        operation: data.operation.unwrap_or_default(),
        side: data.side.unwrap_or_default(),
        price: data.price.unwrap_or_default(),
        size: parse_f64(&data.size),
        smart_depth: data.is_smart_depth.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests;
