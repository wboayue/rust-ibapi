use anyhow::Result;

use crate::{client::ResponseMessage, market_data::RealTimeBar};

pub fn decode_realtime_bar(message: &mut ResponseMessage) -> Result<RealTimeBar> {
    message.skip(); // message type

    let _message_version = message.next_int()?;
    let _request_id = message.next_int()?;
    let date = message.next_long()?; // long, convert to date
    let open = message.next_double()?;
    let high = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    Ok(RealTimeBar {
        date: date.to_string(),
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    })
}
