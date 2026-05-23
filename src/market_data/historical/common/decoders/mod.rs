use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use time_tz::{PrimitiveDateTimeExt, Tz};

use crate::common::timezone::find_timezone;
use crate::messages::ResponseMessage;
use crate::Error;

use crate::market_data::historical::{
    Bar, HistogramEntry, HistoricalData, Schedule, Session, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint,
};

pub(crate) fn decode_head_timestamp(message: &ResponseMessage) -> Result<OffsetDateTime, Error> {
    parse_unix_seconds_str(&decode_head_timestamp_proto(message.require_proto()?)?)
}

fn parse_unix_seconds_str(s: &str) -> Result<OffsetDateTime, Error> {
    let mk_err = |e: &dyn std::fmt::Display| Error::parse_field(s, format!("invalid unix-second timestamp: {e}"));
    let secs: i64 = s.parse().map_err(|e: std::num::ParseIntError| mk_err(&e))?;
    OffsetDateTime::from_unix_timestamp(secs).map_err(|e| mk_err(&e))
}

pub(crate) fn decode_historical_data(message: &ResponseMessage) -> Result<HistoricalData, Error> {
    let bars = decode_historical_data_proto(message.require_proto()?)?;
    // start/end always come on the separate HistoricalDataEnd message at floor 210.
    Ok(HistoricalData {
        start: OffsetDateTime::UNIX_EPOCH,
        end: OffsetDateTime::UNIX_EPOCH,
        bars,
    })
}

pub(crate) fn decode_historical_data_end(message: &ResponseMessage) -> Result<(OffsetDateTime, OffsetDateTime), Error> {
    decode_historical_data_end_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_schedule(message: &ResponseMessage) -> Result<Schedule, Error> {
    decode_historical_schedule_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_ticks_bid_ask(message: &ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
    decode_historical_ticks_bid_ask_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_ticks_mid_point(message: &ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
    decode_historical_ticks_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_ticks_last(message: &ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
    decode_historical_ticks_last_proto(message.require_proto()?)
}

pub(crate) fn decode_histogram_data(message: &ResponseMessage) -> Result<Vec<HistogramEntry>, Error> {
    decode_histogram_data_proto(message.require_proto()?)
}

/// Decode a HistoricalDataUpdate message (message type 90).
///
/// Sent when historical data is requested with `keepUpToDate=true`. IBKR
/// emits updates approximately every 4-6 seconds for the current (incomplete) bar.
pub(crate) fn decode_historical_data_update(message: &ResponseMessage) -> Result<Bar, Error> {
    decode_historical_data_update_proto(message.require_proto()?)
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

/// Parses "YYYYMMDD HH:MM:SS TZ" (single space + embedded timezone) — the
/// shape `HistoricalDataEnd` carries in its `start_date_str` / `end_date_str`.
fn parse_date_with_tz(text: &str) -> Result<OffsetDateTime, Error> {
    let fmt = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let (datetime_part, tz_name) = text
        .rsplit_once(' ')
        .ok_or_else(|| Error::parse_field(text, "expected 'YYYYMMDD HH:MM:SS TZ'"))?;
    let tz = parse_time_zone(tz_name.trim())?;
    let dt = PrimitiveDateTime::parse(datetime_part, fmt)?;
    Ok(dt.assume_timezone(tz).unwrap())
}

// === Protobuf decoders ===

use prost::Message;

use crate::proto;
use crate::proto::decoders::{parse_f64 as parse_str_f64, parse_i32 as parse_str_i32, ts};

pub(crate) fn decode_historical_data_proto(bytes: &[u8]) -> Result<Vec<Bar>, Error> {
    let msg = proto::HistoricalData::decode(bytes)?;
    Ok(msg.historical_data_bars.iter().map(|b| decode_historical_data_bar(b, -1)).collect())
}

fn decode_historical_data_bar(b: &proto::HistoricalDataBar, default_count: i32) -> Bar {
    let date = b
        .date
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    Bar {
        date,
        open: b.open.unwrap_or_default(),
        high: b.high.unwrap_or_default(),
        low: b.low.unwrap_or_default(),
        close: b.close.unwrap_or_default(),
        volume: parse_str_f64(&b.volume),
        wap: parse_str_f64(&b.wap),
        count: b.bar_count.unwrap_or(default_count),
    }
}

pub(crate) fn decode_head_timestamp_proto(bytes: &[u8]) -> Result<String, Error> {
    let msg = proto::HeadTimestamp::decode(bytes)?;
    Ok(msg.head_timestamp.unwrap_or_default())
}

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

pub(crate) fn decode_historical_ticks_last_proto(bytes: &[u8]) -> Result<(Vec<TickLast>, bool), Error> {
    let msg = proto::HistoricalTicksLast::decode(bytes)?;

    let ticks = msg
        .historical_ticks_last
        .into_iter()
        .map(|t| {
            let attr = t.tick_attrib_last;
            TickLast {
                timestamp: ts(t.time.unwrap_or_default()),
                tick_attribute_last: TickAttributeLast {
                    past_limit: attr.as_ref().and_then(|a| a.past_limit).unwrap_or_default(),
                    unreported: attr.and_then(|a| a.unreported).unwrap_or_default(),
                },
                price: t.price.unwrap_or_default(),
                size: parse_str_i32(&t.size),
                exchange: t.exchange.unwrap_or_default(),
                special_conditions: t.special_conditions.unwrap_or_default(),
            }
        })
        .collect();

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

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

pub(crate) fn decode_historical_data_end_proto(bytes: &[u8]) -> Result<(OffsetDateTime, OffsetDateTime), Error> {
    let p = proto::HistoricalDataEnd::decode(bytes)?;
    let start = parse_date_with_tz(p.start_date_str.as_deref().unwrap_or(""))?;
    let end = parse_date_with_tz(p.end_date_str.as_deref().unwrap_or(""))?;
    Ok((start, end))
}

pub(crate) fn decode_histogram_data_proto(bytes: &[u8]) -> Result<Vec<HistogramEntry>, Error> {
    let p = proto::HistogramData::decode(bytes)?;
    Ok(p.histogram_data_entries
        .into_iter()
        .map(|e| HistogramEntry {
            price: e.price.unwrap_or_default(),
            size: parse_str_i32(&e.size),
        })
        .collect())
}

pub(crate) fn decode_historical_schedule_proto(bytes: &[u8]) -> Result<Schedule, Error> {
    let p = proto::HistoricalSchedule::decode(bytes)?;
    let time_zone_name = p.time_zone.unwrap_or_default();
    let time_zone = parse_time_zone(&time_zone_name)?;

    let sessions = p
        .historical_sessions
        .into_iter()
        .map(|s| -> Result<Session, Error> {
            Ok(Session {
                start: parse_schedule_date_time(s.start_date_time.as_deref().unwrap_or(""), time_zone)?,
                end: parse_schedule_date_time(s.end_date_time.as_deref().unwrap_or(""), time_zone)?,
                reference: parse_schedule_date(s.ref_date.as_deref().unwrap_or(""))?,
            })
        })
        .collect::<Result<Vec<Session>, Error>>()?;

    Ok(Schedule {
        start: parse_schedule_date_time(p.start_date_time.as_deref().unwrap_or(""), time_zone)?,
        end: parse_schedule_date_time(p.end_date_time.as_deref().unwrap_or(""), time_zone)?,
        time_zone: time_zone_name,
        sessions,
    })
}

pub(crate) fn decode_historical_data_update_proto(bytes: &[u8]) -> Result<Bar, Error> {
    let p = proto::HistoricalDataUpdate::decode(bytes)?;
    Ok(decode_historical_data_bar(&p.historical_data_bar.unwrap_or_default(), 0))
}

#[cfg(test)]
mod tests;
