use super::*;
use time::macros::datetime;
use time_tz::{self, PrimitiveDateTimeExt};

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
