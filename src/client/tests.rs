use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

#[test]
fn parse_connection_time() {
    let example = "20230405 22:20:39 PST";
    let connection_time = super::parse_connection_time(example);

    let la = timezones::db::america::LOS_ANGELES;
    if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
        assert_eq!(connection_time, other);
    }
}
