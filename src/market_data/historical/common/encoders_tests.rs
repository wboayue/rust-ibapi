use super::*;
use time::macros::datetime;
use time_tz::{self, PrimitiveDateTimeExt};

#[test]
fn historical_data_request_without_chart_options_preserves_wire_format() {
    use crate::testdata::builders::market_data::historical_data_request;
    use crate::testdata::builders::RequestEncoder;

    let contract = Contract::stock("SPY").build();
    let duration = Duration::days(1);
    for (end_date, bar_size, what_to_show, use_rth, keep_up_to_date) in [
        (Some(datetime!(2023-04-15 10:00 UTC)), BarSize::Hour, WhatToShow::Trades, false, false),
        (None, BarSize::Hour, WhatToShow::Trades, true, true),
        (None, BarSize::Day, WhatToShow::Schedule, true, false),
    ] {
        let actual = encode_request_historical_data(1, &contract, end_date, duration, bar_size, what_to_show, use_rth, keep_up_to_date)
            .expect("historical data request should encode");
        let expected = historical_data_request()
            .request_id(1)
            .contract(&contract)
            .end_date(end_date)
            .duration(duration)
            .bar_size(bar_size)
            .what_to_show(what_to_show)
            .use_rth(use_rth)
            .keep_up_to_date(keep_up_to_date)
            .encode_request();

        assert_eq!(actual, expected, "historical data request bytes must remain unchanged");
    }
}

#[test]
fn test_encode_interval() {
    let ny = time_tz::timezones::db::america::NEW_YORK;

    let empty_end: Option<OffsetDateTime> = None;
    let valid_end_utc: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00 UTC));
    let valid_end_ny: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00).assume_timezone(ny).unwrap());

    assert_eq!(empty_end.to_field(), "", "encode empty end");
    assert_eq!(valid_end_utc.to_field(), "20230415 10:00:00 UTC", "encode end utc");
    assert_eq!(valid_end_ny.to_field(), "20230415 14:00:00 UTC", "encode end from America/NewYork");
}
