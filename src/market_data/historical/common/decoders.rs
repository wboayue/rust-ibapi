use time::macros::{format_description, time};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use time_tz::{OffsetDateTimeExt, PrimitiveDateTimeExt, Tz};

use crate::common::timezone::find_timezone;
use crate::messages::ResponseMessage;
use crate::{server_versions, Error};

use crate::market_data::historical::{
    Bar, HistogramEntry, HistoricalData, Schedule, Session, TickAttributeBidAsk, TickAttributeLast, TickBidAsk, TickLast, TickMidpoint,
};

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

fn parse_time_zone(name: &str) -> &Tz {
    let zones = find_timezone(name);
    if zones.is_empty() {
        panic!("timezone not found for: {name}")
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
mod tests {
    use super::*;
    use time::macros::{date, datetime};
    use time_tz;

    #[test]
    fn test_decode_head_timestamp() {
        let mut message = ResponseMessage::from("88\09000\01560346200\0");

        let head_timestamp = super::decode_head_timestamp(&mut message).expect("error decoding trade tick");

        assert_eq!(head_timestamp, datetime!(2019-06-12 13:30).assume_utc(), "head_timestamp");
    }

    #[test]
    fn test_decode_historical_schedule() {
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        let mut message =
            ResponseMessage::from("106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0");

        let schedule = decode_historical_schedule(&mut message).expect("error decoding historical schedule");

        assert_eq!(
            schedule.start,
            datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(),
            "schedule.start"
        );
        assert_eq!(
            schedule.end,
            datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
            "schedule.end"
        );
        assert_eq!(schedule.time_zone, "US/Eastern", "schedule.time_zone");

        assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
        assert_eq!(schedule.sessions[0].reference, date!(2023 - 04 - 14), "schedule.sessions[0].reference");
        assert_eq!(
            schedule.sessions[0].start,
            datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(),
            "schedule.sessions[0].start"
        );
        assert_eq!(
            schedule.sessions[0].end,
            datetime!(2023-04-14 16:00:00.0).assume_timezone(time_zone).unwrap(),
            "schedule.sessions[0].end"
        );
    }

    #[test]
    fn test_decode_historical_data() {
        let mut message = ResponseMessage::from("17\09000\020230413  16:31:22\020230415  16:31:22\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0");

        let server_version = server_versions::HISTORICAL_SCHEDULE;
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        let historical_data = decode_historical_data(server_version, time_zone, &mut message).expect("error decoding historical data");

        assert_eq!(
            historical_data.start,
            datetime!(2023-04-13 16:31:22).assume_timezone(time_zone).unwrap(),
            "historical_data.start"
        );
        assert_eq!(
            historical_data.end,
            datetime!(2023-04-15 16:31:22).assume_timezone(time_zone).unwrap(),
            "historical_data.end"
        );

        assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");
        assert_eq!(
            historical_data.bars[0].date,
            datetime!(2023-04-13 0:00:00 UTC),
            "historical_data.bars[0].date"
        );
        assert_eq!(historical_data.bars[0].open, 182.94, "historical_data.bars[0].open");
        assert_eq!(historical_data.bars[0].high, 186.50, "historical_data.bars[0].high");
        assert_eq!(historical_data.bars[0].low, 180.94, "historical_data.bars[0].low");
        assert_eq!(historical_data.bars[0].close, 185.90, "historical_data.bars[0].close");
        assert_eq!(historical_data.bars[0].volume, 948837.22, "historical_data.bars[0].volume");
        assert_eq!(historical_data.bars[0].wap, 184.869, "historical_data.bars[0].wap");
        assert_eq!(historical_data.bars[0].count, 324891, "historical_data.bars[0].count");
    }

    #[test]
    fn test_decode_historical_tick_bid_ask() {
        let sample_message = "97\09000\04\01681133399\00\011.63\011.83\02800\0100\01681133400\00\011.63\011.83\02800\0200\01681133400\00\011.63\011.72\02800\0100\01681133400\00\011.63\011.83\02800\0200\01\0";
        let mut message = ResponseMessage::from(sample_message);

        let (ticks, done) = decode_historical_ticks_bid_ask(&mut message).unwrap();

        assert_eq!(ticks.len(), 4, "ticks.len()");
        assert!(done, "done");

        assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:29:59 UTC), "ticks[0].timestamp");
        assert_eq!(
            ticks[0].tick_attribute_bid_ask,
            TickAttributeBidAsk {
                bid_past_low: false,
                ask_past_high: false
            },
            "ticks[0].tick_attribute_bid_ask"
        );
        assert_eq!(ticks[0].price_bid, 11.63, "ticks[0].price_bid");
        assert_eq!(ticks[0].price_ask, 11.83, "ticks[0].price_ask");
        assert_eq!(ticks[0].size_bid, 2800, "ticks[0].size_bid");
        assert_eq!(ticks[0].size_ask, 100, "ticks[0].size_ask");

        assert_eq!(ticks[3].timestamp, datetime!(2023-04-10 13:30:00 UTC), "ticks[0].timestamp");
        assert_eq!(
            ticks[3].tick_attribute_bid_ask,
            TickAttributeBidAsk {
                bid_past_low: false,
                ask_past_high: false
            },
            "ticks[0].tick_attribute_bid_ask"
        );
        assert_eq!(ticks[3].price_bid, 11.63, "ticks[0].price_bid");
        assert_eq!(ticks[3].price_ask, 11.83, "ticks[0].price_ask");
        assert_eq!(ticks[3].size_bid, 2800, "ticks[0].size_bid");
        assert_eq!(ticks[3].size_ask, 200, "ticks[0].size_ask");
    }

    #[test]
    fn test_decode_historical_tick_last() {
        let sample_message = "98\09000\07\01681133400\00\011.63\024547\0ISLAND\0 O X\01681133400\02\011.73\01\0DRCTEDGE\0   I\01681133401\00\011.63\0179\0FINRA\0\01681133401\02\011.73\01\0FINRA\0   I\01681133402\02\011.63\01\0FINRA\0 4 I\01681133402\02\011.73\01\0FINRA\0   I\01681133402\02\011.73\01\0FINRA\0   I\01\0";
        let mut message = ResponseMessage::from(sample_message);

        let (ticks, done) = decode_historical_ticks_last(&mut message).unwrap();

        assert_eq!(ticks.len(), 7, "ticks.len()");
        assert!(done, "done");

        assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:30:0 UTC), "ticks[0].timestamp");
        assert_eq!(
            ticks[0].tick_attribute_last,
            TickAttributeLast {
                past_limit: false,
                unreported: false
            },
            "ticks[0].tick_attribute_last"
        );
        assert_eq!(ticks[0].price, 11.63, "ticks[0].price");
        assert_eq!(ticks[0].size, 24547, "ticks[0].size");
        assert_eq!(ticks[0].exchange, "ISLAND", "ticks[0].exchange");
        assert_eq!(ticks[0].special_conditions, " O X", "ticks[0].special_conditions");

        assert_eq!(ticks[6].timestamp, datetime!(2023-04-10 13:30:02 UTC), "ticks[6].timestamp");
        assert_eq!(
            ticks[6].tick_attribute_last,
            TickAttributeLast {
                past_limit: false,
                unreported: true
            },
            "ticks[6].tick_attribute_last"
        );
        assert_eq!(ticks[6].price, 11.73, "ticks[6].price");
        assert_eq!(ticks[6].size, 1, "ticks[6].size");
        assert_eq!(ticks[6].exchange, "FINRA", "ticks[6].exchange");
        assert_eq!(ticks[6].special_conditions, "   I", "ticks[6].special_conditions");
    }

    #[test]
    fn test_decode_historical_tick_midpoint() {
        let sample_message = "96\09000\024\01681133398\00\091.36\00\01681133400\00\091.355\00\01681133400\00\091.35\00\01681133400\00\091.345\00\01681133400\00\091.35\00\01681133400\00\091.355\00\01681133400\00\091.35\00\01681133400\00\091.34\00\01681133400\00\091.345\00\01681133400\00\091.34\00\01681133400\00\091.345\00\01681133400\00\091.34\00\01681133400\00\091.335\00\01681133400\00\091.33\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.315\00\01681133400\00\091.32\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.315\00\01681133400\00\091.31\00\01\0";
        let mut message = ResponseMessage::from(sample_message);

        let (ticks, done) = decode_historical_ticks_mid_point(&mut message).unwrap();

        assert_eq!(ticks.len(), 24, "ticks.len()");
        assert!(done, "done");

        assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:29:58 UTC), "ticks[0].timestamp");
        assert_eq!(ticks[0].price, 91.36, "ticks[0].price");
        assert_eq!(ticks[0].size, 0, "ticks[0].size");

        assert_eq!(ticks[23].timestamp, datetime!(2023-04-10 13:30:00 UTC), "ticks[0].timestamp");
        assert_eq!(ticks[23].price, 91.31, "ticks[0].price");
        assert_eq!(ticks[23].size, 0, "ticks[0].size");
    }

    #[test]
    fn test_decode_historical_data_update() {
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        // Message format: message_type|request_id|bar_count|timestamp|open|high|low|close|volume|wap|count
        let mut message = ResponseMessage::from("90\09000\0-1\01681133400\0185.50\0186.00\0185.00\0185.75\01000.5\0185.625\0150\0");

        let bar = decode_historical_data_update(time_zone, &mut message).expect("error decoding historical data update");

        assert_eq!(bar.date, datetime!(2023-04-10 13:30:00 UTC), "bar.date");
        assert_eq!(bar.open, 185.50, "bar.open");
        assert_eq!(bar.high, 186.00, "bar.high");
        assert_eq!(bar.low, 185.00, "bar.low");
        assert_eq!(bar.close, 185.75, "bar.close");
        assert_eq!(bar.volume, 1000.5, "bar.volume");
        assert_eq!(bar.wap, 185.625, "bar.wap");
        assert_eq!(bar.count, 150, "bar.count");
    }

    #[test]
    fn test_decode_historical_data_update_without_count() {
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        // Message without count field (optional in streaming updates)
        let mut message = ResponseMessage::from("90\09000\0-1\01681133400\0185.50\0186.00\0185.00\0185.75\01000.5\0185.625\0");

        let bar = decode_historical_data_update(time_zone, &mut message).expect("error decoding historical data update");

        assert_eq!(bar.date, datetime!(2023-04-10 13:30:00 UTC), "bar.date");
        assert_eq!(bar.open, 185.50, "bar.open");
        assert_eq!(bar.high, 186.00, "bar.high");
        assert_eq!(bar.low, 185.00, "bar.low");
        assert_eq!(bar.close, 185.75, "bar.close");
        assert_eq!(bar.volume, 1000.5, "bar.volume");
        assert_eq!(bar.wap, 185.625, "bar.wap");
        assert_eq!(bar.count, 0, "bar.count should default to 0 when missing");
    }
}
