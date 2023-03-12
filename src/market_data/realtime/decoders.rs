use anyhow::Result;
use time::OffsetDateTime;

use crate::{
    client::ResponseMessage,
    market_data::{BidAsk, BidAskAttribute, RealTimeBar, Trade, TradeAttribute, MidPoint},
};

pub fn decode_realtime_bar(message: &mut ResponseMessage) -> Result<RealTimeBar> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // message request id

    let date = message.next_long()?; // long, convert to date
    let open = message.next_double()?;
    let high = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    let timestamp = OffsetDateTime::from_unix_timestamp(date).unwrap();

    Ok(RealTimeBar {
        date: timestamp,
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    })
}

pub fn trade_tick(message: &mut ResponseMessage) -> Result<Trade> {
    message.skip(); // message type
    message.skip(); // message request id

    //https://github.com/InteractiveBrokers/tws-api/blob/255ec4bcfd0060dea38d4dff8c46293179b0f79c/source/csharpclient/client/EDecoder.cs#L507

    let date = message.next_long()?; // long, convert to date
    let price = message.next_double()?;
    let size = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    let timestamp = OffsetDateTime::from_unix_timestamp(date).unwrap();

    Ok(Trade {
        tick_type: "todo".to_owned(),
        time: OffsetDateTime::now_utc(),
        price: 0.0,
        size: 0,
        trade_attribute: TradeAttribute {
            past_limit: false,
            unreported: false,
        },
        exchange: "todo".to_owned(),
        special_conditions: "todo".to_owned(),
    })
}

pub fn bid_ask_tick(message: &mut ResponseMessage) -> Result<BidAsk> {
    message.skip(); // message type
    message.skip(); // message request id

    //https://github.com/InteractiveBrokers/tws-api/blob/255ec4bcfd0060dea38d4dff8c46293179b0f79c/source/csharpclient/client/EDecoder.cs#L507

    let date = message.next_long()?; // long, convert to date
    let price = message.next_double()?;
    let size = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    let timestamp = OffsetDateTime::from_unix_timestamp(date).unwrap();

    Ok(BidAsk {
        time: OffsetDateTime::now_utc(),
        bid_price: 0.0,
        ask_price: 0.0,
        bid_size: 0,
        ask_size: 0,
        bid_ask_attribute: BidAskAttribute {
            bid_past_low: todo!(),
            ask_past_high: todo!(),
        },
    })
}

pub fn mid_point_tick(message: &mut ResponseMessage) -> Result<MidPoint> {
    message.skip(); // message type
    message.skip(); // message request id

    //https://github.com/InteractiveBrokers/tws-api/blob/255ec4bcfd0060dea38d4dff8c46293179b0f79c/source/csharpclient/client/EDecoder.cs#L507

    let date = message.next_long()?; // long, convert to date
    let price = message.next_double()?;
    let size = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    let timestamp = OffsetDateTime::from_unix_timestamp(date).unwrap();

    Ok(MidPoint {  })
}
