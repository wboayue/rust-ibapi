use crate::Error;
use crate::{messages::ResponseMessage, server_versions};

use super::{Bar, BidAsk, BidAskAttribute, DepthMarketDataDescription, MarketDepth, MarketDepthL2, MidPoint, Trade, TradeAttribute};

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

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn decode_trade() {
        let mut message = ResponseMessage::from("99\09000\01\01678740829\03895.25\07\02\0\0\0");

        let results = decode_trade_tick(&mut message);

        if let Ok(trade) = results {
            assert_eq!(trade.tick_type, "1", "trade.tick_type");
            assert_eq!(trade.time, OffsetDateTime::from_unix_timestamp(1678740829).unwrap(), "trade.time");
            assert_eq!(trade.price, 3895.25, "trade.price");
            assert_eq!(trade.size, 7, "trade.size");
            assert_eq!(trade.trade_attribute.past_limit, false, "trade.trade_attribute.past_limit");
            assert_eq!(trade.trade_attribute.unreported, true, "trade.trade_attribute.unreported");
            assert_eq!(trade.exchange, "", "trade.exchange");
            assert_eq!(trade.special_conditions, "", "trade.special_conditions");
        } else if let Err(err) = results {
            assert!(false, "error decoding trade tick: {err}");
        }
    }

    #[test]
    fn decode_bid_ask() {
        let mut message = ResponseMessage::from("99\09000\03\01678745793\03895.50\03896.00\09\011\01\0");

        let results = decode_bid_ask_tick(&mut message);

        if let Ok(bid_ask) = results {
            assert_eq!(bid_ask.time, OffsetDateTime::from_unix_timestamp(01678745793).unwrap(), "bid_ask.time");
            assert_eq!(bid_ask.bid_price, 3895.5, "bid_ask.bid_price");
            assert_eq!(bid_ask.ask_price, 3896.0, "bid_ask.ask_price");
            assert_eq!(bid_ask.bid_size, 9, "bid_ask.bid_size");
            assert_eq!(bid_ask.ask_size, 11, "bid_ask.ask_size");
            assert_eq!(bid_ask.bid_ask_attribute.bid_past_low, true, "bid_ask.bid_ask_attribute.bid_past_low");
            assert_eq!(bid_ask.bid_ask_attribute.ask_past_high, false, "bid_ask.bid_ask_attribute.ask_past_high");
        } else if let Err(err) = results {
            assert!(false, "error decoding trade tick: {err}");
        }
    }

    #[test]
    fn decode_mid_point() {
        let mut message = ResponseMessage::from("99\09000\04\01678746113\03896.875\0");

        let results = decode_mid_point_tick(&mut message);

        if let Ok(mid_point) = results {
            assert_eq!(mid_point.time, OffsetDateTime::from_unix_timestamp(1678746113).unwrap(), "mid_point.time");
            assert_eq!(mid_point.mid_point, 3896.875, "mid_point.mid_point");
        } else if let Err(err) = results {
            assert!(false, "error decoding mid point tick: {err}");
        }
    }
}
