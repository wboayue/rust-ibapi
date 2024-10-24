use crate::contracts::tick_types::TickType;
use crate::Error;
use crate::{messages::ResponseMessage, server_versions};

use super::{
    Bar, BidAsk, BidAskAttribute, DepthMarketDataDescription, MarketDepth, MarketDepthL2, MidPoint, TickEFP, TickGeneric, TickOptionComputation,
    TickPrice, TickRequestParameters, TickSize, TickString, Trade, TradeAttribute,
};

#[cfg(test)]
mod tests;

pub(super) fn decode_realtime_bar(message: &mut ResponseMessage) -> Result<Bar, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    Ok(Bar {
        date: message.next_date_time()?,
        open: message.next_double()?,
        high: message.next_double()?,
        low: message.next_double()?,
        close: message.next_double()?,
        volume: message.next_double()?,
        wap: message.next_double()?,
        count: message.next_int()?,
    })
}

pub(super) fn decode_trade_tick(message: &mut ResponseMessage) -> Result<Trade, Error> {
    message.skip(); // message type
    message.skip(); // message request id

    let tick_type = message.next_int()?;
    if !(tick_type == 1 || tick_type == 2) {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }

    let date = message.next_date_time()?;
    let price = message.next_double()?;
    let size = message.next_long()?;
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

pub(super) fn decode_bid_ask_tick(message: &mut ResponseMessage) -> Result<BidAsk, Error> {
    message.skip(); // message type
    message.skip(); // message request id

    let tick_type = message.next_int()?;
    if tick_type != 3 {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }

    let date = message.next_date_time()?;
    let bid_price = message.next_double()?;
    let ask_price = message.next_double()?;
    let bid_size = message.next_long()?;
    let ask_size = message.next_long()?;
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

pub(super) fn decode_mid_point_tick(message: &mut ResponseMessage) -> Result<MidPoint, Error> {
    message.skip(); // message type
    message.skip(); // message request id

    let tick_type = message.next_int()?;
    if tick_type != 4 {
        return Err(Error::Simple(format!("Unexpected tick_type: {tick_type}")));
    }

    Ok(MidPoint {
        time: message.next_date_time()?,
        mid_point: message.next_double()?,
    })
}

pub(super) fn decode_market_depth(message: &mut ResponseMessage) -> Result<MarketDepth, Error> {
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

pub(super) fn decode_market_depth_l2(server_version: i32, message: &mut ResponseMessage) -> Result<MarketDepthL2, Error> {
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

pub(super) fn decode_market_depth_exchanges(server_version: i32, message: &mut ResponseMessage) -> Result<Vec<DepthMarketDataDescription>, Error> {
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

pub(super) fn decode_tick_price(message: &mut ResponseMessage) -> Result<TickPrice, Error> {
    message.skip(); // message type
    message.skip(); // message request id

    Ok(TickPrice {})
}

pub(super) fn decode_tick_size(message: &mut ResponseMessage) -> Result<TickSize, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    Ok(TickSize {
        tick_type: TickType::from(message.next_int()?),
        size: message.next_double()?,
    })
}

pub(super) fn decode_tick_string(message: &mut ResponseMessage) -> Result<TickString, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    Ok(TickString {
        tick_type: TickType::from(message.next_int()?),
        value: message.next_string()?,
    })
}

pub(super) fn decode_tick_efp(message: &mut ResponseMessage) -> Result<TickEFP, Error> {
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

pub(super) fn decode_tick_generic(message: &mut ResponseMessage) -> Result<TickGeneric, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    Ok(TickGeneric {
        tick_type: TickType::from(message.next_int()?),
        value: message.next_double()?,
    })
}

pub(super) fn decode_tick_option_computation(server_version: i32, message: &mut ResponseMessage) -> Result<TickOptionComputation, Error> {
    // use crate::contracts::decoders::decode_option_computation();

    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    Ok(TickOptionComputation {})
}

pub(super) fn decode_tick_request_parameters(message: &mut ResponseMessage) -> Result<TickRequestParameters, Error> {
    message.skip(); // message type
    message.skip(); // message request id

    Ok(TickRequestParameters {
        min_tick: message.next_double()?,
        bbo_exchange: message.next_string()?,
        snapshot_permissions: message.next_int()?,
    })
}
