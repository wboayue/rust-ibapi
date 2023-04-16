use time::macros::{format_description, time};
use time::{Date, PrimitiveDateTime};
use time_tz::{timezones, OffsetDateTimeExt, PrimitiveDateTimeExt, Tz};

use super::*;

pub(super) fn decode_head_timestamp(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let head_timestamp = message.next_date_time()?;

    Ok(head_timestamp)
}

pub(super) fn decode_historical_data(server_version: i32, time_zone: &Tz, message: &mut ResponseMessage) -> Result<HistoricalData, Error> {
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

pub(super) fn decode_historical_schedule(message: &mut ResponseMessage) -> Result<HistoricalSchedule, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let start = message.next_string()?;
    let end = message.next_string()?;
    let time_zone_name = message.next_string()?;

    let time_zone = parse_time_zone(&time_zone_name);

    let sessions_count = message.next_int()?;
    let mut sessions = Vec::<HistoricalSession>::with_capacity(sessions_count as usize);
    for _ in 0..sessions_count {
        let session_start = message.next_string()?;
        let session_end = message.next_string()?;
        let session_reference = message.next_string()?;

        sessions.push(HistoricalSession {
            start: parse_schedule_date_time(&session_start, time_zone)?,
            end: parse_schedule_date_time(&session_end, time_zone)?,
            reference: parse_schedule_date(&session_reference)?,
        })
    }

    Ok(HistoricalSchedule {
        start: parse_schedule_date_time(&start, time_zone)?,
        end: parse_schedule_date_time(&end, time_zone)?,
        time_zone: time_zone_name,
        sessions,
    })
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
mod tests {
    use time::macros::{date, datetime};
    use time_tz;

    use super::*;

    #[test]
    fn test_decode_head_timestamp() {
        let mut message = ResponseMessage::from("88\09000\01560346200\0");

        let results = super::decode_head_timestamp(&mut message);

        if let Ok(head_timestamp) = results {
            assert_eq!(head_timestamp, datetime!(2019-06-12 13:30).assume_utc(), "head_timestamp");
        } else if let Err(err) = results {
            assert!(false, "error decoding trade tick: {err}");
        }
    }

    #[test]
    fn test_decode_historical_schedule() {
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        let mut message = ResponseMessage::from("106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0");

        let results = decode_historical_schedule(&mut message);

        if let Ok(schedule) = results {
            assert_eq!(schedule.start, datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(), "schedule.start");
            assert_eq!(schedule.end, datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(), "schedule.end");
            assert_eq!(schedule.time_zone, "US/Eastern", "schedule.time_zone");

            assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
            assert_eq!(schedule.sessions[0].reference, date!(2023-04-14), "schedule.sessions[0].reference");
            assert_eq!(schedule.sessions[0].start, datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(), "schedule.sessions[0].start");
            assert_eq!(schedule.sessions[0].end, datetime!(2023-04-14 16:00:00.0).assume_timezone(time_zone).unwrap(), "schedule.sessions[0].end");
        } else if let Err(err) = results {
            assert!(false, "error decoding historical schedule {err}");
        }
    }

    #[test]
    fn test_decode_historical_data() {
        let mut message = ResponseMessage::from("17\09000\020230413  16:31:22\020230415  16:31:22\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0");

        let server_version = server_versions::HISTORICAL_SCHEDULE;
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        let results = decode_historical_data(server_version, time_zone, &mut message);

        if let Ok(historical_data) = results {
            assert_eq!(historical_data.start, datetime!(2023-04-13 16:31:22).assume_timezone(time_zone).unwrap(), "historical_data.start");
            assert_eq!(historical_data.end, datetime!(2023-04-15 16:31:22).assume_timezone(time_zone).unwrap(), "historical_data.end");

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
        } else if let Err(err) = results {
            assert!(false, "error decoding historical data {err}");
        }
    }
}
