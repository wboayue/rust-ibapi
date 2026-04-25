use time::macros::{format_description, time};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use time_tz::{OffsetDateTimeExt, PrimitiveDateTimeExt, Tz};

use crate::common::timezone::find_timezone;
use crate::messages::ResponseMessage;
use crate::{server_versions, Error};

use crate::market_data::historical::{
    Bar, HistogramEntry, HistoricalData, Schedule, Session, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint,
};

pub(crate) fn decode_head_timestamp(message: &mut ResponseMessage, time_zone: Option<&Tz>) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let head_timestamp = message.next_date_time_with_timezone(time_zone)?;

    Ok(head_timestamp)
}

pub(crate) fn decode_historical_data(server_version: i32, time_zone: &Tz, message: &mut ResponseMessage) -> Result<HistoricalData, Error> {
    message.skip(); // message type

    let mut message_version = i32::MAX;
    if server_version < server_versions::SYNT_REALTIME_BARS {
        message_version = message.next_int()?;
    }

    message.skip(); // request_id

    let mut start = OffsetDateTime::now_utc();
    let mut end = OffsetDateTime::now_utc();
    if message_version > 2 && server_version < server_versions::HISTORICAL_DATA_END {
        start = parse_date(&message.next_string()?, time_zone)?;
        end = parse_date(&message.next_string()?, time_zone)?;
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

pub(crate) fn decode_historical_data_end(
    server_version: i32,
    time_zone: &Tz,
    message: &mut ResponseMessage,
) -> Result<(OffsetDateTime, OffsetDateTime), Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let start_str = message.next_string()?;
    let end_str = message.next_string()?;

    if server_version >= server_versions::HISTORICAL_DATA_END {
        let start = parse_date_with_tz(&start_str)?;
        let end = parse_date_with_tz(&end_str)?;
        Ok((start, end))
    } else {
        let start = parse_date(&start_str, time_zone)?;
        let end = parse_date(&end_str, time_zone)?;
        Ok((start, end))
    }
}

pub(crate) fn decode_historical_schedule(message: &mut ResponseMessage) -> Result<Schedule, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let start = message.next_string()?;
    let end = message.next_string()?;
    let time_zone_name = message.next_string()?;

    let time_zone = parse_time_zone(&time_zone_name)?;

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

/// Decode a HistoricalDataUpdate message (message type 90).
///
/// This message is sent when historical data is requested with keepUpToDate=true.
/// IBKR sends updates approximately every 4-6 seconds for the current (incomplete) bar.
///
/// Message format:
/// - message_type (90)
/// - request_id
/// - bar_count (always -1 for streaming updates)
/// - date (unix timestamp as string)
/// - open
/// - high
/// - low
/// - close
/// - volume
/// - wap
/// - count
pub(crate) fn decode_historical_data_update(time_zone: &Tz, message: &mut ResponseMessage) -> Result<Bar, Error> {
    message.skip(); // message type
    message.skip(); // request_id
    message.skip(); // bar_count (always -1 for updates)

    let date = message.next_string()?;
    let open = message.next_double()?;
    let high = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    // count field is optional in streaming updates - may not be present
    let count = message.next_int().unwrap_or(0);

    Ok(Bar {
        date: parse_bar_date(&date, time_zone)?,
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    })
}

fn parse_time_zone(name: &str) -> Result<&'static Tz, Error> {
    let zones = find_timezone(name);
    if zones.is_empty() {
        return Err(Error::UnsupportedTimeZone(name.to_string()));
    }
    Ok(zones[0])
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

/// Parses "YYYYMMDD  HH:MM:SS" (double space, no timezone).
fn parse_date(text: &str, time_zone: &Tz) -> Result<OffsetDateTime, Error> {
    let fmt = format_description!("[year][month][day]  [hour]:[minute]:[second]");
    let dt = PrimitiveDateTime::parse(text, fmt)?;
    Ok(dt.assume_timezone(time_zone).unwrap())
}

/// Parses "YYYYMMDD HH:MM:SS TZ" (single space + embedded timezone).
fn parse_date_with_tz(text: &str) -> Result<OffsetDateTime, Error> {
    let fmt = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let (datetime_part, tz_name) = text
        .rsplit_once(' ')
        .ok_or_else(|| Error::Simple(format!("expected 'YYYYMMDD HH:MM:SS TZ', got: {text}")))?;
    let tz = parse_time_zone(tz_name.trim())?;
    let dt = PrimitiveDateTime::parse(datetime_part, fmt)?;
    Ok(dt.assume_timezone(tz).unwrap())
}

fn parse_bar_date(text: &str, time_zone: &Tz) -> Result<OffsetDateTime, Error> {
    if text.len() == 8 {
        let date_format = format_description!("[year][month][day]");
        let bar_date = Date::parse(text, date_format)?;
        let bar_date = bar_date.with_time(time!(00:00));

        Ok(bar_date.assume_timezone_utc(time_tz::timezones::db::UTC))
    } else {
        let timestamp: i64 = text
            .parse()
            .map_err(|e: std::num::ParseIntError| Error::Simple(format!("parse error: \"{text}\" - {e}")))?;
        let date_utc = OffsetDateTime::from_unix_timestamp(timestamp).unwrap();
        Ok(date_utc.to_timezone(time_zone))
    }
}

// === Protobuf decoders ===

use prost::Message;

use crate::proto;
use crate::proto::decoders::{parse_f64 as parse_str_f64, parse_i32 as parse_str_i32, ts};

#[allow(dead_code)]
pub(crate) fn decode_historical_data_proto(bytes: &[u8]) -> Result<Vec<Bar>, Error> {
    let msg = proto::HistoricalData::decode(bytes)?;

    let bars = msg
        .historical_data_bars
        .iter()
        .map(|b| {
            let date_str = b.date.as_deref().unwrap_or_default();
            let date = date_str
                .parse::<i64>()
                .map(|ts_val| OffsetDateTime::from_unix_timestamp(ts_val).unwrap_or(OffsetDateTime::UNIX_EPOCH))
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);

            Bar {
                date,
                open: b.open.unwrap_or_default(),
                high: b.high.unwrap_or_default(),
                low: b.low.unwrap_or_default(),
                close: b.close.unwrap_or_default(),
                volume: parse_str_f64(&b.volume),
                wap: parse_str_f64(&b.wap),
                count: b.bar_count.unwrap_or(-1),
            }
        })
        .collect();

    Ok(bars)
}

#[allow(dead_code)]
pub(crate) fn decode_head_timestamp_proto(bytes: &[u8]) -> Result<String, Error> {
    let msg = proto::HeadTimestamp::decode(bytes)?;
    Ok(msg.head_timestamp.unwrap_or_default())
}

#[allow(dead_code)]
pub(crate) fn decode_real_time_bar_proto(bytes: &[u8]) -> Result<crate::market_data::realtime::Bar, Error> {
    let msg = proto::RealTimeBarTick::decode(bytes)?;

    Ok(crate::market_data::realtime::Bar {
        date: OffsetDateTime::from_unix_timestamp(msg.time.unwrap_or_default()).unwrap_or(OffsetDateTime::UNIX_EPOCH),
        open: msg.open.unwrap_or_default(),
        high: msg.high.unwrap_or_default(),
        low: msg.low.unwrap_or_default(),
        close: msg.close.unwrap_or_default(),
        volume: parse_str_f64(&msg.volume),
        wap: parse_str_f64(&msg.wap),
        count: msg.count.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_historical_ticks_proto(bytes: &[u8]) -> Result<(Vec<TickMidpoint>, bool), Error> {
    let msg = proto::HistoricalTicks::decode(bytes)?;

    let ticks = msg
        .historical_ticks
        .iter()
        .map(|t| TickMidpoint {
            timestamp: ts(t.time.unwrap_or_default()),
            price: t.price.unwrap_or_default(),
            size: parse_str_i32(&t.size),
        })
        .collect();

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

#[allow(dead_code)]
pub(crate) fn decode_historical_ticks_last_proto(bytes: &[u8]) -> Result<(Vec<TickLast>, bool), Error> {
    let msg = proto::HistoricalTicksLast::decode(bytes)?;

    let ticks = msg
        .historical_ticks_last
        .iter()
        .map(|t| {
            let attr = t.tick_attrib_last.as_ref();
            TickLast {
                timestamp: ts(t.time.unwrap_or_default()),
                tick_attribute_last: TickAttributeLast {
                    past_limit: attr.and_then(|a| a.past_limit).unwrap_or_default(),
                    unreported: attr.and_then(|a| a.unreported).unwrap_or_default(),
                },
                price: t.price.unwrap_or_default(),
                size: parse_str_i32(&t.size),
                exchange: t.exchange.clone().unwrap_or_default(),
                special_conditions: t.special_conditions.clone().unwrap_or_default(),
            }
        })
        .collect();

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

#[allow(dead_code)]
pub(crate) fn decode_historical_ticks_bid_ask_proto(bytes: &[u8]) -> Result<(Vec<TickBidAsk>, bool), Error> {
    let msg = proto::HistoricalTicksBidAsk::decode(bytes)?;

    let ticks = msg
        .historical_ticks_bid_ask
        .iter()
        .map(|t| {
            let attr = t.tick_attrib_bid_ask.as_ref();
            TickBidAsk {
                timestamp: ts(t.time.unwrap_or_default()),
                tick_attribute_bid_ask: TickAttributeBidAsk {
                    ask_past_high: attr.and_then(|a| a.ask_past_high).unwrap_or_default(),
                    bid_past_low: attr.and_then(|a| a.bid_past_low).unwrap_or_default(),
                },
                price_bid: t.price_bid.unwrap_or_default(),
                price_ask: t.price_ask.unwrap_or_default(),
                size_bid: parse_str_i32(&t.size_bid),
                size_ask: parse_str_i32(&t.size_ask),
            }
        })
        .collect();

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

#[cfg(test)]
mod tests;
