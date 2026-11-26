use super::*;

#[test]
fn what_to_show_to_string() {
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
fn what_to_show_parse() {
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

/// `(variant, wire text, 5.x variant name)` for every variant, written independently of the
/// `Display` and `FromStr` matches.
const BAR_SIZES: [(BarSize, &str, &str); 22] = [
    (BarSize::Sec, "1 secs", "Sec"),
    (BarSize::Sec5, "5 secs", "Sec5"),
    (BarSize::Sec10, "10 secs", "Sec10"),
    (BarSize::Sec15, "15 secs", "Sec15"),
    (BarSize::Sec30, "30 secs", "Sec30"),
    (BarSize::Min, "1 min", "Min"),
    (BarSize::Min2, "2 mins", "Min2"),
    (BarSize::Min3, "3 mins", "Min3"),
    (BarSize::Min4, "4 mins", "Min4"),
    (BarSize::Min5, "5 mins", "Min5"),
    (BarSize::Min10, "10 mins", "Min10"),
    (BarSize::Min15, "15 mins", "Min15"),
    (BarSize::Min20, "20 mins", "Min20"),
    (BarSize::Min30, "30 mins", "Min30"),
    (BarSize::Hour, "1 hour", "Hour"),
    (BarSize::Hour2, "2 hours", "Hour2"),
    (BarSize::Hour3, "3 hours", "Hour3"),
    (BarSize::Hour4, "4 hours", "Hour4"),
    (BarSize::Hour8, "8 hours", "Hour8"),
    (BarSize::Day, "1 day", "Day"),
    (BarSize::Week, "1 week", "Week"),
    (BarSize::Month, "1 month", "Month"),
];

/// Exhaustive, so a new variant fails to compile here until it gets a `BAR_SIZES` row.
fn bar_size_index(bar_size: BarSize) -> usize {
    match bar_size {
        BarSize::Sec => 0,
        BarSize::Sec5 => 1,
        BarSize::Sec10 => 2,
        BarSize::Sec15 => 3,
        BarSize::Sec30 => 4,
        BarSize::Min => 5,
        BarSize::Min2 => 6,
        BarSize::Min3 => 7,
        BarSize::Min4 => 8,
        BarSize::Min5 => 9,
        BarSize::Min10 => 10,
        BarSize::Min15 => 11,
        BarSize::Min20 => 12,
        BarSize::Min30 => 13,
        BarSize::Hour => 14,
        BarSize::Hour2 => 15,
        BarSize::Hour3 => 16,
        BarSize::Hour4 => 17,
        BarSize::Hour8 => 18,
        BarSize::Day => 19,
        BarSize::Week => 20,
        BarSize::Month => 21,
    }
}

#[test]
fn bar_size_table_covers_every_variant() {
    for (i, (bar_size, _, _)) in BAR_SIZES.iter().enumerate() {
        assert_eq!(bar_size_index(*bar_size), i, "{bar_size:?} row out of place");
    }
}

#[test]
fn bar_size_round_trips_its_wire_text() {
    use crate::common::test_utils::wire_enum::check_serde_form;

    for (bar_size, wire, name) in BAR_SIZES {
        assert_eq!(bar_size.to_string(), wire, "Display for {bar_size:?}");
        assert_eq!(bar_size.to_field(), wire, "ToField for {bar_size:?}");
        check_serde_form(&bar_size, serde_json::json!(wire));
        // FromStr is case-insensitive on both the wire text and the 5.x variant name, so 5.x
        // JSON (`"Min5"`) still loads.
        for input in [
            wire.to_string(),
            wire.to_uppercase(),
            name.to_string(),
            name.to_uppercase(),
            name.to_lowercase(),
        ] {
            assert_eq!(input.parse::<BarSize>(), Ok(bar_size), "FromStr({input})");
        }
        assert_eq!(
            serde_json::from_value::<BarSize>(serde_json::json!(name)).unwrap(),
            bar_size,
            "5.x JSON {name}"
        );
    }
    assert!("5 minutes".parse::<BarSize>().is_err());
    assert!(serde_json::from_str::<BarSize>("\"\"").is_err());
}

#[test]
fn what_to_show_serde_is_the_wire_string() {
    use crate::common::test_utils::wire_enum::check_serde_form;

    check_serde_form(&WhatToShow::BidAsk, serde_json::json!("BID_ASK"));
    check_serde_form(&WhatToShow::AdjustedLast, serde_json::json!("ADJUSTED_LAST"));
    // The 5.x variant name is not a wire string.
    assert!(serde_json::from_str::<WhatToShow>("\"BidAsk\"").is_err());
}

#[test]
fn duration() {
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
fn duration_parse() {
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
fn historical_parse_error_display() {
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
fn bar_timestamp_from_str_date() {
    let ts: BarTimestamp = "20230411".parse().unwrap();
    assert_eq!(ts, BarTimestamp::Date(time::macros::date!(2023 - 04 - 11)));
    assert!(ts.is_date());
    assert!(!ts.is_date_time());
}

#[test]
fn bar_timestamp_from_str_datetime() {
    let ts: BarTimestamp = "1681133400".parse().unwrap();
    assert_eq!(ts, BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 13:30:00 UTC)));
    assert!(ts.is_date_time());
    assert!(!ts.is_date());
}

#[test]
fn bar_timestamp_from_str_invalid() {
    assert!("not-a-date".parse::<BarTimestamp>().is_err());
    assert!("".parse::<BarTimestamp>().is_err());
}

#[test]
fn bar_timestamp_display_roundtrip_date() {
    let ts = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    assert_eq!(ts.to_string(), "20230411");
    let round_tripped: BarTimestamp = ts.to_string().parse().unwrap();
    assert_eq!(round_tripped, ts);
}

#[test]
fn bar_timestamp_display_roundtrip_datetime() {
    let dt = time::macros::datetime!(2023-04-10 13:30:00 UTC);
    let ts = BarTimestamp::DateTime(dt);
    assert_eq!(ts.to_string(), "1681133400");
    let round_tripped: BarTimestamp = ts.to_string().parse().unwrap();
    assert_eq!(round_tripped, ts);
}

#[test]
fn bar_timestamp_from_date() {
    let d = time::macros::date!(2023 - 04 - 11);
    let ts: BarTimestamp = d.into();
    assert_eq!(ts, BarTimestamp::Date(d));
}

#[test]
fn bar_timestamp_from_offset_date_time() {
    let dt = time::macros::datetime!(2023-04-10 13:30:00 UTC);
    let ts: BarTimestamp = dt.into();
    assert_eq!(ts, BarTimestamp::DateTime(dt));
}

#[test]
fn bar_timestamp_ord_same_variant() {
    let a = BarTimestamp::Date(time::macros::date!(2023 - 04 - 10));
    let b = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    assert!(a < b);

    let c = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 13:00:00 UTC));
    let d = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 14:00:00 UTC));
    assert!(c < d);
}

#[test]
fn bar_timestamp_ord_cross_variant() {
    let date = BarTimestamp::Date(time::macros::date!(2023 - 04 - 11));
    let before = BarTimestamp::DateTime(time::macros::datetime!(2023-04-10 23:59:59 UTC));
    let after = BarTimestamp::DateTime(time::macros::datetime!(2023-04-11 00:00:01 UTC));
    assert!(before < date);
    assert!(date < after);
}

#[test]
fn bar_timestamp_sort() {
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

fn trim_bar(date: BarTimestamp) -> Bar {
    Bar {
        date,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: Some(0.0),
        wap: Some(0.0),
        count: Some(0),
    }
}

fn trim_data(dates: &[BarTimestamp]) -> HistoricalData {
    HistoricalData {
        start: time::macros::datetime!(2026-04-01 0:00 UTC),
        end: time::macros::datetime!(2026-05-01 0:00 UTC),
        bars: dates.iter().copied().map(trim_bar).collect(),
    }
}

fn bar_dates(data: &HistoricalData) -> Vec<BarTimestamp> {
    data.bars.iter().map(|bar| bar.date).collect()
}

#[test]
fn trim_to_keeps_start_and_drops_end() {
    use time::macros::datetime;
    let (start, end) = (datetime!(2026-04-15 0:00 UTC), datetime!(2026-04-16 0:00 UTC));
    let dates = [datetime!(2026-04-14 23:59 UTC), start, datetime!(2026-04-15 13:30 UTC), end].map(BarTimestamp::from);

    let trimmed = trim_data(&dates).trim_to(start, end);

    assert_eq!(bar_dates(&trimmed), vec![BarTimestamp::from(start), dates[2]]);
    assert_eq!((trimmed.start, trimmed.end), (start, end));
}

#[test]
fn trim_to_outside_every_bar_is_empty() {
    use time::macros::datetime;
    let (start, end) = (datetime!(2026-04-20 0:00 UTC), datetime!(2026-04-21 0:00 UTC));

    let trimmed = trim_data(&[BarTimestamp::from(datetime!(2026-04-15 13:30 UTC))]).trim_to(start, end);

    assert!(trimmed.bars.is_empty());
    assert_eq!((trimmed.start, trimmed.end), (start, end));
}

#[test]
fn trim_to_containing_every_bar_keeps_them() {
    use time::macros::datetime;
    let dates = [datetime!(2026-04-15 13:30 UTC), datetime!(2026-04-15 19:00 UTC)].map(BarTimestamp::from);
    let data = trim_data(&dates);

    let trimmed = data.clone().trim_to(data.start, data.end);

    assert_eq!(trimmed, data);
}

/// A daily bar's date counts as midnight UTC.
#[test]
fn trim_to_daily_bars() {
    use time::macros::{date, datetime};
    let dates = [date!(2026 - 04 - 14), date!(2026 - 04 - 15), date!(2026 - 04 - 16)].map(BarTimestamp::from);

    let trimmed = trim_data(&dates).trim_to(datetime!(2026-04-15 0:00 UTC), datetime!(2026-04-16 0:00 UTC));

    assert_eq!(bar_dates(&trimmed), vec![BarTimestamp::from(date!(2026 - 04 - 15))]);
}

// Destructures every field without `..`, so a new field fails to compile here until its
// default is chosen.
#[test]
fn bar_default_is_zero_at_epoch_with_no_trade_data() {
    let Bar {
        date,
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    } = Bar::default();
    assert_eq!(date, BarTimestamp::DateTime(OffsetDateTime::UNIX_EPOCH));
    assert_eq!((open, high, low, close), (0.0, 0.0, 0.0, 0.0));
    assert_eq!((volume, wap, count), (None, None, None));
}
