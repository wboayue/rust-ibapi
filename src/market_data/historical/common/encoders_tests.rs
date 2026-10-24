use super::*;
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::messages::{OutgoingMessages, MESSAGE_ID_LEN};
use prost::Message;
use time::macros::datetime;
use time_tz::{self, PrimitiveDateTimeExt};

#[test]
fn encode_interval() {
    let ny = time_tz::timezones::db::america::NEW_YORK;

    let empty_end: Option<OffsetDateTime> = None;
    let valid_end_utc: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00 UTC));
    let valid_end_ny: Option<OffsetDateTime> = Some(datetime!(2023-04-15 10:00).assume_timezone(ny).unwrap());

    assert_eq!(empty_end.to_field(), "", "encode empty end");
    assert_eq!(valid_end_utc.to_field(), "20230415 10:00:00 UTC", "encode end utc");
    assert_eq!(valid_end_ny.to_field(), "20230415 14:00:00 UTC", "encode end from America/NewYork");
}

#[test]
fn encode_cancel_histogram_data_works() {
    let bytes = encode_cancel_histogram_data(3000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelHistogramData);
    let req = crate::proto::CancelHistogramData::decode(&bytes[MESSAGE_ID_LEN..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
}

#[test]
fn encode_cancel_head_timestamp_works() {
    let bytes = encode_cancel_head_timestamp(3000).unwrap();
    assert_proto_msg_id(&bytes, OutgoingMessages::CancelHeadTimestamp);
    let req = crate::proto::CancelHeadTimestamp::decode(&bytes[MESSAGE_ID_LEN..]).unwrap();
    assert_eq!(req.req_id, Some(3000));
}
