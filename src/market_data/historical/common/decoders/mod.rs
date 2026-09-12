use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};
use time_tz::Tz;

use crate::common::timezone::{find_timezone, resolve_local};
use crate::messages::ResponseMessage;
use crate::Error;

use crate::market_data::historical::{
    Bar, BarTimestamp, HistogramEntry, HistoricalData, Schedule, Session, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint,
};

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

pub(crate) fn decode_historical_ticks_bid_ask(message: &ResponseMessage) -> Result<(Vec<TickBidAsk>, bool), Error> {
    decode_historical_ticks_bid_ask_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_ticks_mid_point(message: &ResponseMessage) -> Result<(Vec<TickMidpoint>, bool), Error> {
    decode_historical_ticks_proto(message.require_proto()?)
}

pub(crate) fn decode_historical_ticks_last(message: &ResponseMessage) -> Result<(Vec<TickLast>, bool), Error> {
    decode_historical_ticks_last_proto(message.require_proto()?)
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

/// `YYYYMMDD-HH:MM:SS`: historical-schedule session bounds, and the zone-less UTC
/// rendering of `HistoricalDataEnd`.
const DASHED_DATE_TIME: &[FormatItem<'static>] = format_description!("[year][month][day]-[hour]:[minute]:[second]");

fn parse_schedule_date_time(text: &str, time_zone: &Tz) -> Result<OffsetDateTime, Error> {
    let schedule_date_time = PrimitiveDateTime::parse(text, DASHED_DATE_TIME)?;
    Ok(resolve_local(schedule_date_time, time_zone))
}

fn parse_yyyymmdd(text: &str) -> Result<Date, Error> {
    Ok(Date::parse(text, format_description!("[year][month][day]"))?)
}

/// Parses the two wire-verified renderings `HistoricalDataEnd` carries in its
/// `start_date_str` / `end_date_str`. The gateway's "Send instrument-specific
/// attributes ... in" setting picks the rendering:
///
/// - instrument timezone (default): `YYYYMMDD HH:MM:SS <zone>`, a wall clock in the
///   named zone. The zone name may contain spaces (`China Standard Time`), so
///   everything after the time is the zone.
/// - UTC format: `YYYYMMDD-HH:MM:SS`, zone-less, the UTC shape documented since
///   TWS 10.17. Captured from a 10.50 gateway in #808.
///
/// Anything else is rejected loudly rather than guessed at: a zone-less wall clock
/// resolved in the wrong zone is a silently wrong instant. Candidate shapes seen on
/// other messages or older APIs are tracked in `plans/historical-data-end-renderings.md`.
fn parse_historical_data_end_timestamp(text: &str) -> Result<OffsetDateTime, Error> {
    let text = text.trim();
    if let Ok(utc) = PrimitiveDateTime::parse(text, DASHED_DATE_TIME) {
        return Ok(utc.assume_utc());
    }
    let mut parts = text.splitn(3, ' ');
    let (Some(date), Some(time), Some(zone_name)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(Error::parse_field(text, "expected 'YYYYMMDD HH:MM:SS <zone>' or 'YYYYMMDD-HH:MM:SS'"));
    };
    // Wall clock before zone, so a malformed clock reports a parse error rather than
    // an unsupported timezone.
    let wall_clock = PrimitiveDateTime::new(parse_yyyymmdd(date)?, Time::parse(time, format_description!("[hour]:[minute]:[second]"))?);
    let zone = parse_time_zone(zone_name.trim())?;
    Ok(resolve_local(wall_clock, zone))
}

// === Protobuf decoders ===

use prost::Message;

use crate::proto;
use crate::proto::decoders::{parse_decimal_or_zero, parse_optional_decimal, ts};

pub(crate) fn decode_historical_data_proto(bytes: &[u8]) -> Result<Vec<Bar>, Error> {
    let msg = proto::HistoricalData::decode(bytes)?;
    // Sized up front rather than `.collect::<Result<Vec<_>, _>>()`: collecting into
    // a Result goes through `process_results`, whose `size_hint` lower bound is 0
    // (it may short-circuit), so the Vec would grow by doubling. Bar responses run
    // to tens of thousands of rows.
    let mut bars = Vec::with_capacity(msg.historical_data_bars.len());
    for bar in &msg.historical_data_bars {
        bars.push(decode_historical_data_bar(bar, -1)?);
    }
    Ok(bars)
}

fn decode_historical_data_bar(b: &proto::HistoricalDataBar, default_count: i32) -> Result<Bar, Error> {
    let date = b
        .date
        .as_deref()
        .and_then(|s| s.parse::<BarTimestamp>().ok())
        .unwrap_or(BarTimestamp::DateTime(OffsetDateTime::UNIX_EPOCH));
    Ok(Bar {
        date,
        open: b.open.unwrap_or_default(),
        high: b.high.unwrap_or_default(),
        low: b.low.unwrap_or_default(),
        close: b.close.unwrap_or_default(),
        volume: parse_decimal_or_zero(b.volume.as_deref())?,
        wap: parse_decimal_or_zero(b.wap.as_deref())?,
        count: b.bar_count.unwrap_or(default_count),
    })
}

pub(crate) fn decode_head_timestamp_proto(msg: crate::proto::HeadTimestamp) -> Result<OffsetDateTime, Error> {
    parse_unix_seconds_str(&msg.head_timestamp.unwrap_or_default())
}

pub(crate) fn decode_historical_ticks_proto(bytes: &[u8]) -> Result<(Vec<TickMidpoint>, bool), Error> {
    let msg = proto::HistoricalTicks::decode(bytes)?;

    let mut ticks = Vec::with_capacity(msg.historical_ticks.len());
    for t in &msg.historical_ticks {
        ticks.push(TickMidpoint {
            timestamp: ts(t.time.unwrap_or_default()),
            price: t.price.unwrap_or_default(),
            size: parse_optional_decimal(t.size.as_deref())?,
        });
    }

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

pub(crate) fn decode_historical_ticks_last_proto(bytes: &[u8]) -> Result<(Vec<TickLast>, bool), Error> {
    let msg = proto::HistoricalTicksLast::decode(bytes)?;

    let mut ticks = Vec::with_capacity(msg.historical_ticks_last.len());
    for t in msg.historical_ticks_last {
        let attr = t.tick_attrib_last;
        ticks.push(TickLast {
            timestamp: ts(t.time.unwrap_or_default()),
            tick_attribute_last: TickAttributeLast {
                past_limit: attr.as_ref().and_then(|a| a.past_limit).unwrap_or_default(),
                unreported: attr.and_then(|a| a.unreported).unwrap_or_default(),
            },
            price: t.price.unwrap_or_default(),
            size: parse_optional_decimal(t.size.as_deref())?,
            exchange: t.exchange.unwrap_or_default(),
            special_conditions: t.special_conditions.unwrap_or_default(),
        });
    }

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

pub(crate) fn decode_historical_ticks_bid_ask_proto(bytes: &[u8]) -> Result<(Vec<TickBidAsk>, bool), Error> {
    let msg = proto::HistoricalTicksBidAsk::decode(bytes)?;

    let mut ticks = Vec::with_capacity(msg.historical_ticks_bid_ask.len());
    for t in &msg.historical_ticks_bid_ask {
        let attr = t.tick_attrib_bid_ask.as_ref();
        ticks.push(TickBidAsk {
            timestamp: ts(t.time.unwrap_or_default()),
            tick_attribute_bid_ask: TickAttributeBidAsk {
                ask_past_high: attr.and_then(|a| a.ask_past_high).unwrap_or_default(),
                bid_past_low: attr.and_then(|a| a.bid_past_low).unwrap_or_default(),
            },
            price_bid: t.price_bid.unwrap_or_default(),
            price_ask: t.price_ask.unwrap_or_default(),
            size_bid: parse_optional_decimal(t.size_bid.as_deref())?,
            size_ask: parse_optional_decimal(t.size_ask.as_deref())?,
        });
    }

    Ok((ticks, msg.is_done.unwrap_or_default()))
}

pub(crate) fn decode_historical_data_end_proto(bytes: &[u8]) -> Result<(OffsetDateTime, OffsetDateTime), Error> {
    let p = proto::HistoricalDataEnd::decode(bytes)?;
    let start = parse_historical_data_end_timestamp(p.start_date_str.as_deref().unwrap_or(""))?;
    let end = parse_historical_data_end_timestamp(p.end_date_str.as_deref().unwrap_or(""))?;
    Ok((start, end))
}

pub(crate) fn decode_histogram_data_proto(p: crate::proto::HistogramData) -> Result<Vec<HistogramEntry>, Error> {
    let mut entries = Vec::with_capacity(p.histogram_data_entries.len());
    for e in &p.histogram_data_entries {
        entries.push(HistogramEntry {
            price: e.price.unwrap_or_default(),
            size: parse_optional_decimal(e.size.as_deref())?,
        });
    }
    Ok(entries)
}

pub(crate) fn decode_historical_schedule_proto(p: crate::proto::HistoricalSchedule) -> Result<Schedule, Error> {
    let time_zone_name = p.time_zone.unwrap_or_default();
    let time_zone = parse_time_zone(&time_zone_name)?;

    let sessions = p
        .historical_sessions
        .into_iter()
        .map(|s| -> Result<Session, Error> {
            Ok(Session {
                start: parse_schedule_date_time(s.start_date_time.as_deref().unwrap_or(""), time_zone)?,
                end: parse_schedule_date_time(s.end_date_time.as_deref().unwrap_or(""), time_zone)?,
                reference: parse_yyyymmdd(s.ref_date.as_deref().unwrap_or(""))?,
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
    decode_historical_data_bar(&p.historical_data_bar.unwrap_or_default(), 0)
}

#[cfg(test)]
mod tests;
