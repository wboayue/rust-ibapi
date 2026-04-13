use prost::Message;

use crate::contracts::decode_option_computation;
use crate::contracts::OptionComputation;
use crate::proto::decoders::optional_f64;
use crate::subscriptions::DecoderContext;
use crate::Error;
use crate::{messages::ResponseMessage, server_versions};

use crate::market_data::realtime::{
    Bar, BidAsk, BidAskAttribute, DepthMarketDataDescription, MarketDepth, MarketDepthL2, MidPoint, TickAttribute, TickEFP, TickGeneric, TickPrice,
    TickPriceSize, TickRequestParameters, TickSize, TickString, TickType, TickTypes, Trade, TradeAttribute,
};

pub(crate) fn decode_realtime_bar(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Bar, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    Ok(Bar {
        date: message.next_date_time_with_timezone(context.time_zone)?,
        open: message.next_double()?,
        high: message.next_double()?,
        low: message.next_double()?,
        close: message.next_double()?,
        volume: message.next_double()?,
        wap: message.next_double()?,
        count: message.next_int()?,
    })
}
pub(crate) fn decode_trade_tick(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Trade, Error> {
    message.skip(); // message type
    message.skip(); // message request id
    let tick_type = message.next_int()?;
    if !(tick_type == 1 || tick_type == 2) {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }
    let date = message.next_date_time_with_timezone(context.time_zone)?;
    let price = message.next_double()?;
    let size = message.next_double()?;
    let mask = message.next_int()?;
    let exchange = message.next_string()?;
    let special_conditions = message.next_string()?;
    Ok(Trade {
        tick_type: tick_type.to_string(),
        time: date,
        price,
        size,
        trade_attribute: TradeAttribute {
            past_limit: mask & 0x1 != 0,
            unreported: mask & 0x2 != 0,
        },
        exchange,
        special_conditions,
    })
}
pub(crate) fn decode_bid_ask_tick(context: &DecoderContext, message: &mut ResponseMessage) -> Result<BidAsk, Error> {
    message.skip(); // message type
    message.skip(); // message request id
    let tick_type = message.next_int()?;
    if tick_type != 3 {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }
    let date = message.next_date_time_with_timezone(context.time_zone)?;
    let bid_price = message.next_double()?;
    let ask_price = message.next_double()?;
    let bid_size = message.next_double()?;
    let ask_size = message.next_double()?;
    let mask = message.next_int()?;
    Ok(BidAsk {
        time: date,
        bid_price,
        ask_price,
        bid_size,
        ask_size,
        bid_ask_attribute: BidAskAttribute {
            bid_past_low: mask & 0x1 != 0,
            ask_past_high: mask & 0x2 != 0,
        },
    })
}
pub(crate) fn decode_mid_point_tick(context: &DecoderContext, message: &mut ResponseMessage) -> Result<MidPoint, Error> {
    message.skip(); // message type
    message.skip(); // message request id
    let tick_type = message.next_int()?;
    if tick_type != 4 {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }
    Ok(MidPoint {
        time: message.next_date_time_with_timezone(context.time_zone)?,
        mid_point: message.next_double()?,
    })
}
pub(crate) fn decode_market_depth(message: &mut ResponseMessage) -> Result<MarketDepth, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    let depth = MarketDepth {
        position: message.next_int()?,
        operation: message.next_int()?,
        side: message.next_int()?,
        price: message.next_double()?,
        size: message.next_double()?,
    };
    Ok(depth)
}
pub(crate) fn decode_market_depth_l2(server_version: i32, message: &mut ResponseMessage) -> Result<MarketDepthL2, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    let mut depth = MarketDepthL2 {
        position: message.next_int()?,
        market_maker: message.next_string()?,
        operation: message.next_int()?,
        side: message.next_int()?,
        price: message.next_double()?,
        size: message.next_double()?,
        ..Default::default()
    };
    if server_version >= server_versions::SMART_DEPTH {
        depth.smart_depth = message.next_bool()?;
    }
    Ok(depth)
}
pub(crate) fn decode_market_depth_exchanges(server_version: i32, message: &mut ResponseMessage) -> Result<Vec<DepthMarketDataDescription>, Error> {
    message.skip(); // message type
    let count = message.next_int()?;
    let mut descriptions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let description = if server_version >= server_versions::SERVICE_DATA_TYPE {
            DepthMarketDataDescription {
                exchange_name: message.next_string()?,
                security_type: message.next_string()?,
                listing_exchange: message.next_string()?,
                service_data_type: message.next_string()?,
                aggregated_group: Some(message.next_string()?),
            }
        } else {
            DepthMarketDataDescription {
                exchange_name: message.next_string()?,
                security_type: message.next_string()?,
                listing_exchange: "".into(),
                service_data_type: if message.next_bool()? { "Deep2".into() } else { "Deep".into() },
                aggregated_group: None,
            }
        };
        descriptions.push(description);
    }
    Ok(descriptions)
}
pub(crate) fn decode_tick_price(server_version: i32, message: &mut ResponseMessage) -> Result<TickTypes, Error> {
    message.skip(); // message type
    let message_version = message.next_int()?;
    message.skip(); // message request id
    let mut tick_price = TickPrice {
        tick_type: TickType::from(message.next_int()?),
        price: message.next_double()?,
        ..Default::default()
    };
    let size = if message_version >= 2 { message.next_double()? } else { f64::MAX };
    if message_version >= 3 {
        let mask = message.next_int()?;
        if server_version >= server_versions::PAST_LIMIT {
            tick_price.attributes.can_auto_execute = mask & 0x1 == 0x1;
            tick_price.attributes.past_limit = mask & 0x2 == 0x2;
            if server_version >= server_versions::PRE_OPEN_BID_ASK {
                tick_price.attributes.pre_open = mask & 0x4 == 0x4;
            }
        }
    }
    let size_tick_type = match tick_price.tick_type {
        TickType::Bid => TickType::BidSize,
        TickType::Ask => TickType::AskSize,
        TickType::Last => TickType::LastSize,
        TickType::DelayedBid => TickType::DelayedBidSize,
        TickType::DelayedAsk => TickType::DelayedAskSize,
        TickType::DelayedLast => TickType::DelayedLastSize,
        _ => TickType::Unknown,
    };
    if message_version < 2 || size_tick_type == TickType::Unknown {
        Ok(TickTypes::Price(tick_price))
    } else {
        Ok(TickTypes::PriceSize(TickPriceSize {
            price_tick_type: tick_price.tick_type,
            price: tick_price.price,
            attributes: tick_price.attributes,
            size_tick_type,
            size,
        }))
    }
}
pub(crate) fn decode_tick_size(message: &mut ResponseMessage) -> Result<TickSize, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    Ok(TickSize {
        tick_type: TickType::from(message.next_int()?),
        size: message.next_double()?,
    })
}
pub(crate) fn decode_tick_string(message: &mut ResponseMessage) -> Result<TickString, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    Ok(TickString {
        tick_type: TickType::from(message.next_int()?),
        value: message.next_string()?,
    })
}
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
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id
    Ok(TickGeneric {
        tick_type: TickType::from(message.next_int()?),
        value: message.next_double()?,
    })
}
pub(crate) fn decode_tick_option_computation(server_version: i32, message: &mut ResponseMessage) -> Result<OptionComputation, Error> {
    decode_option_computation(server_version, message)
}
pub(crate) fn decode_tick_request_parameters(message: &mut ResponseMessage) -> Result<TickRequestParameters, Error> {
    message.skip(); // message type
    message.skip(); // message request id
    Ok(TickRequestParameters {
        min_tick: message.next_double()?,
        bbo_exchange: message.next_string()?,
        snapshot_permissions: message.next_int()?,
    })
}

// === Protobuf decoders ===

#[allow(dead_code)]
pub(crate) fn decode_tick_price_proto(bytes: &[u8]) -> Result<TickTypes, Error> {
    let msg = crate::proto::TickPrice::decode(bytes)?;

    let tick_type = TickType::from(msg.tick_type.unwrap_or_default());
    let price = msg.price.unwrap_or_default();
    let size: f64 = msg.size.as_deref().and_then(|s| s.parse().ok()).unwrap_or(f64::MAX);
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

    if size_tick_type == TickType::Unknown || size == f64::MAX {
        Ok(TickTypes::Price(TickPrice {
            tick_type,
            price,
            attributes,
        }))
    } else {
        Ok(TickTypes::PriceSize(TickPriceSize {
            price_tick_type: tick_type,
            price,
            attributes,
            size_tick_type,
            size,
        }))
    }
}

#[allow(dead_code)]
pub(crate) fn decode_tick_size_proto(bytes: &[u8]) -> Result<TickSize, Error> {
    let msg = crate::proto::TickSize::decode(bytes)?;

    Ok(TickSize {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        size: msg.size.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_tick_string_proto(bytes: &[u8]) -> Result<TickString, Error> {
    let msg = crate::proto::TickString::decode(bytes)?;

    Ok(TickString {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        value: msg.value.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_tick_generic_proto(bytes: &[u8]) -> Result<TickGeneric, Error> {
    let msg = crate::proto::TickGeneric::decode(bytes)?;

    Ok(TickGeneric {
        tick_type: TickType::from(msg.tick_type.unwrap_or_default()),
        value: msg.value.unwrap_or_default(),
    })
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn decode_market_depth_proto(bytes: &[u8]) -> Result<MarketDepth, Error> {
    let msg = crate::proto::MarketDepth::decode(bytes)?;

    let data = msg.market_depth_data.ok_or_else(|| Error::Simple("missing market_depth_data".into()))?;

    Ok(MarketDepth {
        position: data.position.unwrap_or_default(),
        operation: data.operation.unwrap_or_default(),
        side: data.side.unwrap_or_default(),
        price: data.price.unwrap_or_default(),
        size: data.size.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_market_depth_l2_proto(bytes: &[u8]) -> Result<MarketDepthL2, Error> {
    let msg = crate::proto::MarketDepthL2::decode(bytes)?;

    let data = msg.market_depth_data.ok_or_else(|| Error::Simple("missing market_depth_data".into()))?;

    Ok(MarketDepthL2 {
        position: data.position.unwrap_or_default(),
        market_maker: data.market_maker.unwrap_or_default(),
        operation: data.operation.unwrap_or_default(),
        side: data.side.unwrap_or_default(),
        price: data.price.unwrap_or_default(),
        size: data.size.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default(),
        smart_depth: data.is_smart_depth.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests;
