use super::*;

#[test]
fn test_bar_size_to_string() {
    assert_eq!("1 secs", BarSize::Sec.to_string());
    assert_eq!("5 secs", BarSize::Sec5.to_string());
    assert_eq!("10 secs", BarSize::Sec10.to_string());
    assert_eq!("15 secs", BarSize::Sec15.to_string());
    assert_eq!("30 secs", BarSize::Sec30.to_string());
    assert_eq!("1 min", BarSize::Min.to_string());
    assert_eq!("2 mins", BarSize::Min2.to_string());
    assert_eq!("3 mins", BarSize::Min3.to_string());
    assert_eq!("4 mins", BarSize::Min4.to_string());
    assert_eq!("5 mins", BarSize::Min5.to_string());
    assert_eq!("10 mins", BarSize::Min10.to_string());
    assert_eq!("15 mins", BarSize::Min15.to_string());
    assert_eq!("20 mins", BarSize::Min20.to_string());
    assert_eq!("30 mins", BarSize::Min30.to_string());
    assert_eq!("1 hour", BarSize::Hour.to_string());
    assert_eq!("2 hours", BarSize::Hour2.to_string());
    assert_eq!("3 hours", BarSize::Hour3.to_string());
    assert_eq!("4 hours", BarSize::Hour4.to_string());
    assert_eq!("8 hours", BarSize::Hour8.to_string());
    assert_eq!("1 day", BarSize::Day.to_string());
    assert_eq!("1 week", BarSize::Week.to_string());
    assert_eq!("1 month", BarSize::Month.to_string());
}

#[test]
fn test_bar_size_parse() {
    assert_eq!("SEC".parse::<BarSize>(), Ok(BarSize::Sec));
    assert_eq!("SEC5".parse::<BarSize>(), Ok(BarSize::Sec5));
    assert_eq!("SEC10".parse::<BarSize>(), Ok(BarSize::Sec10));
    assert_eq!("SEC15".parse::<BarSize>(), Ok(BarSize::Sec15));
    assert_eq!("SEC30".parse::<BarSize>(), Ok(BarSize::Sec30));
    assert_eq!("MIN".parse::<BarSize>(), Ok(BarSize::Min));
    assert_eq!("MIN2".parse::<BarSize>(), Ok(BarSize::Min2));
    assert_eq!("MIN3".parse::<BarSize>(), Ok(BarSize::Min3));
    assert_eq!("MIN4".parse::<BarSize>(), Ok(BarSize::Min4));
    assert_eq!("MIN5".parse::<BarSize>(), Ok(BarSize::Min5));
    assert_eq!("MIN10".parse::<BarSize>(), Ok(BarSize::Min10));
    assert_eq!("MIN15".parse::<BarSize>(), Ok(BarSize::Min15));
    assert_eq!("MIN20".parse::<BarSize>(), Ok(BarSize::Min20));
    assert_eq!("MIN30".parse::<BarSize>(), Ok(BarSize::Min30));
    assert_eq!("HOUR".parse::<BarSize>(), Ok(BarSize::Hour));
    assert_eq!("HOUR2".parse::<BarSize>(), Ok(BarSize::Hour2));
    assert_eq!("HOUR3".parse::<BarSize>(), Ok(BarSize::Hour3));
    assert_eq!("HOUR4".parse::<BarSize>(), Ok(BarSize::Hour4));
    assert_eq!("HOUR8".parse::<BarSize>(), Ok(BarSize::Hour8));
    assert_eq!("DAY".parse::<BarSize>(), Ok(BarSize::Day));
    assert_eq!("WEEK".parse::<BarSize>(), Ok(BarSize::Week));
    assert_eq!("MONTH".parse::<BarSize>(), Ok(BarSize::Month));
}

#[test]
fn test_what_to_show_to_string() {
    assert_eq!("TRADES", WhatToShow::Trades.to_string());
    assert_eq!("MIDPOINT", WhatToShow::MidPoint.to_string());
    assert_eq!("BID", WhatToShow::Bid.to_string());
    assert_eq!("ASK", WhatToShow::Ask.to_string());
    assert_eq!("BID_ASK", WhatToShow::BidAsk.to_string());
    assert_eq!("AGGTRADES", WhatToShow::AggTrades.to_string());
    assert_eq!("HISTORICAL_VOLATILITY", WhatToShow::HistoricalVolatility.to_string());
    assert_eq!("OPTION_IMPLIED_VOLATILITY", WhatToShow::OptionImpliedVolatility.to_string());
    assert_eq!("FEE_RATE", WhatToShow::FeeRate.to_string());
    assert_eq!("SCHEDULE", WhatToShow::Schedule.to_string());
    assert_eq!("ADJUSTED_LAST", WhatToShow::AdjustedLast.to_string());
}

#[test]
fn test_what_to_show_parse() {
    assert_eq!("TRADES".parse::<WhatToShow>(), Ok(WhatToShow::Trades));
    assert_eq!("MIDPOINT".parse::<WhatToShow>(), Ok(WhatToShow::MidPoint));
    assert_eq!("BID".parse::<WhatToShow>(), Ok(WhatToShow::Bid));
    assert_eq!("ASK".parse::<WhatToShow>(), Ok(WhatToShow::Ask));
    assert_eq!("BID_ASK".parse::<WhatToShow>(), Ok(WhatToShow::BidAsk));
    assert_eq!("AGGTRADES".parse::<WhatToShow>(), Ok(WhatToShow::AggTrades));
    assert_eq!("HISTORICAL_VOLATILITY".parse::<WhatToShow>(), Ok(WhatToShow::HistoricalVolatility));
    assert_eq!("OPTION_IMPLIED_VOLATILITY".parse::<WhatToShow>(), Ok(WhatToShow::OptionImpliedVolatility));
    assert_eq!("FEE_RATE".parse::<WhatToShow>(), Ok(WhatToShow::FeeRate));
    assert_eq!("SCHEDULE".parse::<WhatToShow>(), Ok(WhatToShow::Schedule));
    assert_eq!("ADJUSTED_LAST".parse::<WhatToShow>(), Ok(WhatToShow::AdjustedLast));
}

#[test]
fn test_duration() {
    assert_eq!(Duration::SECOND.to_field(), "1 S");
    assert_eq!(Duration::DAY.to_field(), "1 D");
    assert_eq!(Duration::WEEK.to_field(), "1 W");
    assert_eq!(Duration::MONTH.to_field(), "1 M");
    assert_eq!(Duration::YEAR.to_field(), "1 Y");

    assert_eq!(2.seconds().to_field(), "2 S");
    assert_eq!(3.days().to_field(), "3 D");
    assert_eq!(4.weeks().to_field(), "4 W");
    assert_eq!(5.months().to_field(), "5 M");
    assert_eq!(6.years().to_field(), "6 Y");
}

#[test]
fn test_duration_parse() {
    assert_eq!("1 S".parse(), Ok(Duration::seconds(1)));
    assert_eq!("2 D".parse(), Ok(Duration::days(2)));
    assert_eq!("3 W".parse(), Ok(Duration::weeks(3)));
    assert_eq!("4 M".parse(), Ok(Duration::months(4)));
    assert_eq!("5 Y".parse(), Ok(Duration::years(5)));

    assert_eq!(
        "".parse::<Duration>(),
        Err(HistoricalParseError::Duration("".to_string(), "Empty string".to_string()))
    );
    assert_eq!(
        "1S".parse::<Duration>(),
        Err(HistoricalParseError::Duration("1S".to_string(), "Missing delimiter".to_string()))
    );
    assert_eq!(
        "1 X".parse::<Duration>(),
        Err(HistoricalParseError::Duration("1 X".to_string(), "Unsupported unit".to_string()))
    );

    let expected_int_error = "abc ".parse::<i32>().unwrap_err();
    assert_eq!(
        "abc ".parse::<Duration>(),
        Err(HistoricalParseError::ParseIntError("ABC".to_string(), expected_int_error))
    );
}

#[test]
fn test_historical_parse_error_display() {
    let expected_int_error = "abc".parse::<i32>().unwrap_err();

    let cases = vec![
        (
            HistoricalParseError::BarSize("invalid".to_string()),
            "Invalid BarSize input 'invalid'".to_string(),
        ),
        (
            HistoricalParseError::Duration("invalid".to_string(), "Empty string".to_string()),
            "Invalid Duration input 'invalid' Empty string".to_string(),
        ),
        (
            HistoricalParseError::Duration("1S".to_string(), "Missing delimiter".to_string()),
            "Invalid Duration input '1S' Missing delimiter".to_string(),
        ),
        (
            HistoricalParseError::Duration("1 X".to_string(), "Unsupported unit".to_string()),
            "Invalid Duration input '1 X' Unsupported unit".to_string(),
        ),
        (
            HistoricalParseError::ParseIntError("abc ".to_string(), expected_int_error),
            "ParseIntError 'abc ' invalid digit found in string".to_string(),
        ),
        (
            HistoricalParseError::WhatToShow("invalid".to_string()),
            "Invalid WhatToShow input 'invalid'".to_string(),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn test_bar_timestamp_from_str_date() {
    let ts: BarTimestamp = "20230411".parse().unwrap();
    assert_eq!(ts, BarTimestamp::Date(time::macros::date!(2023 - 04 - 11)));
    assert!(ts.is_date());
    assert!(!ts.is_date_time());
}

#[test]
fn test_bar_timestamp_from_str_datetime() {
    let ts: BarTimestamp = "1681133400".parse().unwrap();
    assert_eq!(ts, BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 13:30:00 UTC)));
    assert!(ts.is_date_time());
    assert!(!ts.is_date());
}

#[test]
fn test_bar_timestamp_from_str_invalid() {
    assert!("not-a-date".parse::<BarTimestamp>().is_err());
    assert!("".parse::<BarTimestamp>().is_err());
}

#[test]
fn test_bar_timestamp_display_roundtrip_date() {
    let ts = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    assert_eq!(ts.to_string(), "20230411");
    let round_tripped: BarTimestamp = ts.to_string().parse().unwrap();
    assert_eq!(round_tripped, ts);
}

#[test]
fn test_bar_timestamp_display_roundtrip_datetime() {
    let dt = time::macros::datetime!(2023-04-10 13:30:00 UTC);
    let ts = BarTimestamp::DateTime(dt);
    assert_eq!(ts.to_string(), "1681133400");
    let round_tripped: BarTimestamp = ts.to_string().parse().unwrap();
    assert_eq!(round_tripped, ts);
}

#[test]
fn test_bar_timestamp_from_date() {
    let d = time::macros::date!(2023 - 04 - 11);
    let ts: BarTimestamp = d.into();
    assert_eq!(ts, BarTimestamp::Date(d));
}

#[test]
fn test_bar_timestamp_from_offset_date_time() {
    let dt = time::macros::datetime!(2023-04-10 13:30:00 UTC);
    let ts: BarTimestamp = dt.into();
    assert_eq!(ts, BarTimestamp::DateTime(dt));
}

#[test]
fn test_bar_timestamp_ord_same_variant() {
    let a = BarTimestamp::Date(time::macros::date!(2023 - 04 - 10));
    let b = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    assert!(a < b);

    let c = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 13:00:00 UTC));
    let d = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 14:00:00 UTC));
    assert!(c < d);
}

#[test]
fn test_bar_timestamp_ord_cross_variant() {
    let date = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    let before = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 23:59:59 UTC));
    let after = BarTimestamp::DateTime(time::macros::datetime!(2023-04-11 00:00:01 UTC));
    assert!(before < date);
    assert!(date < after);
}

#[test]
fn test_bar_timestamp_sort() {
    let mut timestamps = [
        BarTimestamp::DateTime(time::macros::datetime!(2023-04-11 12:00:00 UTC)),
        BarTimestamp::Date(time::macros::date!(2023 - 04 - 10)),
        BarTimestamp::DateTime(time::macros::datetime!(2023-04-09 08:00:00 UTC)),
    ];
    timestamps.sort();
    assert_eq!(timestamps[0], BarTimestamp::DateTime(time::macros::datetime!(2023-04-09 08:00:00 UTC)));
    assert_eq!(timestamps[1], BarTimestamp::Date(time::macros::date!(2023 - 04 - 10)));
    assert_eq!(timestamps[2], BarTimestamp::DateTime(time::macros::datetime!(2023-04-11 12:00:00 UTC)));
}
