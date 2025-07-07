use time::macros::{format_description, time};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, OffsetDateTimeExt, PrimitiveDateTimeExt, Tz};

use crate::messages::ResponseMessage;
use crate::{server_versions, Error};

use crate::market_data::historical::{Bar, HistogramEntry, HistoricalData, Schedule, Session, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint};

pub(crate) fn decode_head_timestamp(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let head_timestamp = message.next_date_time()?;

    Ok(head_timestamp)
}

pub(crate) fn decode_historical_data(server_version: i32, time_zone: &Tz, message: &mut ResponseMessage) -> Result<HistoricalData, Error> {
    message.skip(); // message type

    let mut message_version = i32::MAX;
    if server_version < server_versions::SYNT_REALTIME_BARS {
        message_version = message.next_int()?;
    }

    message.skip(); // request_id

    let slice_format = format_description!("[year][month][day]  [hour]:[minute]:[second]");

    let mut start = OffsetDateTime::now_utc();
    let mut end = OffsetDateTime::now_utc();
    if message_version > 2 {
        start = PrimitiveDateTime::parse(&message.next_string()?, slice_format)?
            .assume_timezone(time_zone)
            .unwrap();
        end = PrimitiveDateTime::parse(&message.next_string()?, slice_format)?
            .assume_timezone(time_zone)
            .unwrap();
    }

    let mut bars = Vec::new();

    let bars_count = message.next_int()?;
    for _ in 0..bars_count {
        let date = message.next_string()?;
        let open = message.next_double()?;
        let high = message.next_double()?;
        let low = message.next_double()?;
        let close = message.next_double()?;
        let volume = message.next_double()?;
        let wap = message.next_double()?;

        if server_version < server_versions::SYNT_REALTIME_BARS {
            // hasGaps
            message.skip();
        }

        let mut bar_count = -1;
        if message_version >= 3 {
            bar_count = message.next_int()?;
        }

        bars.push(Bar {
            date: parse_bar_date(&date, time_zone)?,
            open,
            high,
            low,
            close,
            volume,
            wap,
            count: bar_count,
        })
    }

    Ok(HistoricalData { start, end, bars })
}

pub(crate) fn decode_historical_schedule(message: &mut ResponseMessage) -> Result<Schedule, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let start = message.next_string()?;
    let end = message.next_string()?;
    let time_zone_name = message.next_string()?;

    let time_zone = parse_time_zone(&time_zone_name);

    let sessions_count = message.next_int()?;
    let mut sessions = Vec::<Session>::with_capacity(sessions_count as usize);
    for _ in 0..sessions_count {
        let session_start = message.next_string()?;
        let session_end = message.next_string()?;
        let session_reference = message.next_string()?;

        sessions.push(Session {
            start: parse_schedule_date_time(&session_start, time_zone)?,
            end: parse_schedule_date_time(&session_end, time_zone)?,
            reference: parse_schedule_date(&session_reference)?,
        })
    }

    Ok(Schedule {
        start: parse_schedule_date_time(&start, time_zone)?,
        end: parse_schedule_date_time(&end, time_zone)?,
        time_zone: time_zone_name,
        sessions,
    })
}

pub(crate) fn decode_historical_ticks_bid_ask(message: &mut ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let number_of_ticks = message.next_int()?;
    let mut ticks = Vec::with_capacity(number_of_ticks as usize);

    for _ in 0..number_of_ticks {
        let timestamp = message.next_date_time()?;

        let mask = message.next_int()?;
        let tick_attribute_bid_ask = TickAttributeBidAsk {
            ask_past_high: (mask & 0x01) == 0x01,
            bid_past_low: (mask & 0x02) == 0x02,
        };

        let price_bid = message.next_double()?;
        let price_ask = message.next_double()?;
        let size_bid = message.next_int()?;
        let size_ask = message.next_int()?;

        ticks.push(TickBidAsk {
            timestamp,
            tick_attribute_bid_ask,
            price_bid,
            price_ask,
            size_bid,
            size_ask,
        });
    }

    let done = message.next_bool()?;

    Ok((ticks, done))
}

pub(crate) fn decode_historical_ticks_mid_point(message: &mut ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let number_of_ticks = message.next_int()?;
    let mut ticks = Vec::with_capacity(number_of_ticks as usize);

    for _ in 0..number_of_ticks {
        let timestamp = message.next_date_time()?;
        message.skip(); // for consistency
        let price = message.next_double()?;
        let size = message.next_int()?;

        ticks.push(TickMidpoint { timestamp, price, size });
    }

    let done = message.next_bool()?;

    Ok((ticks, done))
}

pub(crate) fn decode_historical_ticks_last(message: &mut ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let number_of_ticks = message.next_int()?;
    let mut ticks = Vec::with_capacity(number_of_ticks as usize);

    for _ in 0..number_of_ticks {
        let timestamp = message.next_date_time()?;

        let mask = message.next_int()?;
        let tick_attribute_last = TickAttributeLast {
            past_limit: (mask & 0x01) == 0x01,
            unreported: (mask & 0x02) == 0x02,
        };

        let price = message.next_double()?;
        let size = message.next_int()?;
        let exchange = message.next_string()?;
        let special_conditions = message.next_string()?;

        ticks.push(TickLast {
            timestamp,
            tick_attribute_last,
            price,
            size,
            exchange,
            special_conditions,
        });
    }

    let done = message.next_bool()?;

    Ok((ticks, done))
}

pub(crate) fn decode_histogram_data(message: &mut ResponseMessage) -> Result<Vec<HistogramEntry>, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let count = message.next_int()?;
    let mut items = Vec::with_capacity(count as usize);

    for _ in 0..count {
        items.push(HistogramEntry {
            price: message.next_double()?,
            size: message.next_int()?,
        });
    }

    Ok(items)
}

fn parse_time_zone(name: &str) -> &Tz {
    let zones = timezones::find_by_name(name);
    if zones.is_empty() {
        panic!("timezone not found for: {}", name)
    }
    zones[0]
}

fn parse_schedule_date_time(text: &str, time_zone: &Tz) -> Result<OffsetDateTime, Error> {
    let schedule_date_time_format = format_description!("[year][month][day]-[hour]:[minute]:[second]");
    let schedule_date_time = PrimitiveDateTime::parse(text, schedule_date_time_format)?;
    Ok(schedule_date_time.assume_timezone(time_zone).unwrap())
}

fn parse_schedule_date(text: &str) -> Result<Date, Error> {
    let schedule_date_format = format_description!("[year][month][day]");
    let schedule_date = Date::parse(text, schedule_date_format)?;
    Ok(schedule_date)
}

fn parse_bar_date(text: &str, time_zone: &Tz) -> Result<OffsetDateTime, Error> {
    if text.len() == 8 {
        let date_format = format_description!("[year][month][day]");
        let bar_date = Date::parse(text, date_format)?;
        let bar_date = bar_date.with_time(time!(00:00));

        Ok(bar_date.assume_timezone_utc(time_tz::timezones::db::UTC))
    } else {
        let timestamp: i64 = text.parse()?;
        let date_utc = OffsetDateTime::from_unix_timestamp(timestamp).unwrap();
        Ok(date_utc.to_timezone(time_zone))
    }
}

#[cfg(test)]
mod tests;
