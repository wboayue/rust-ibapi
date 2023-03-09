use anyhow::Result;
use time::OffsetDateTime;

use crate::{client::ResponseMessage, market_data::RealTimeBar};

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
