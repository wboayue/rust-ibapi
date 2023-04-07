use time::macros::datetime;
use time_tz::TimeZone;
use time_tz::{timezones, OffsetDateTimeExt, OffsetResult, PrimitiveDateTimeExt};

// #[test]
// fn parse_connection_time() {
//     let connection_time = "20230405 22:20:39 PST";
//     let datetime = DateTime::parse_from_str(connection_time, "%Y%m%d %H:%M:%S %Z").unwrap();
//     assert_eq!(datetime.to_string(), "");
// }

#[test]
fn parse_connection_time() {
    let example = "20230405 22:20:39 PST";
    let connection_time = super::parse_connection_time(example);

    let la = timezones::db::america::LOS_ANGELES;
    if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
        assert_eq!(connection_time, other);
    }
}

//time-tz = "1.0.2"
