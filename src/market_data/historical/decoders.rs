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

    // #[test]
    fn test_decode_historical_schedule() {
        let mut message = ResponseMessage::from("88\09000\01560346200\0");

        let results = decode_historical_schedule(&mut message);

        if let Ok(schedule) = results {
            assert_eq!(schedule.start, datetime!(2019-06-12 13:30 UTC), "schedule.start");
            assert_eq!(schedule.end, datetime!(2019-06-12 13:30 UTC), "schedule.end");
            assert_eq!(schedule.time_zone, "ES", "schedule.time_zone");

            assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
            assert_eq!(schedule.sessions[0].reference, date!(2019 - 06 - 12), "schedule.sessions[0].reference");
            assert_eq!(schedule.sessions[0].start, datetime!(2019-06-12 13:30 UTC), "schedule.sessions[0].start");
            assert_eq!(schedule.sessions[0].end, datetime!(2019-06-12 13:30 UTC), "schedule.sessions[0].end");
        } else if let Err(err) = results {
            assert!(false, "error decoding historical schedule {err}");
        }
    }

    // #[test]
    fn test_decode_historical_data() {
        let mut message = ResponseMessage::from("88\09000\01560346200\0");

        let server_version = server_versions::ACCOUNT_SUMMARY;
        let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

        let results = decode_historical_data(server_version, time_zone, &mut message);

        if let Ok(historical_data) = results {
            assert_eq!(historical_data.start, datetime!(2019-06-12 13:30 UTC), "historical_data.start");
            assert_eq!(historical_data.end, datetime!(2019-06-12 13:30 UTC), "historical_data.end");

            assert_eq!(historical_data.bars.len(), 1, "historical_data.bars.len()");
            assert_eq!(
                historical_data.bars[0].date,
                datetime!(2019-06-12 13:30).assume_timezone(time_zone).unwrap(),
                "historical_data.bars[0].date"
            );
            assert_eq!(historical_data.bars[0].open, 10.0, "historical_data.bars[0].open");
            assert_eq!(historical_data.bars[0].high, 10.3, "historical_data.bars[0].high");
            assert_eq!(historical_data.bars[0].low, 12.0, "historical_data.bars[0].low");
            assert_eq!(historical_data.bars[0].close, 23.0, "historical_data.bars[0].close");
            assert_eq!(historical_data.bars[0].volume, 23.0, "historical_data.bars[0].volume");
            assert_eq!(historical_data.bars[0].wap, 23.0, "historical_data.bars[0].wap");
            assert_eq!(historical_data.bars[0].count, 23, "historical_data.bars[0].count");
        } else if let Err(err) = results {
            assert!(false, "error decoding historical data {err}");
        }
    }
}
