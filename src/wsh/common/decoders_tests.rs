use super::*;
use crate::common::test_utils::helpers::assert_rejects_text_framing;
use crate::messages::IncomingMessages;

#[test]
fn decode_wsh_metadata_proto_works() {
    let proto_msg = crate::proto::WshMetaData {
        req_id: Some(1),
        data_json: Some(r#"{"key":"value"}"#.into()),
    };

    let result = decode_wsh_metadata_proto(proto_msg).unwrap();
    assert_eq!(result.data_json, r#"{"key":"value"}"#);
}

#[test]
fn decode_wsh_event_data_proto_works() {
    let proto_msg = crate::proto::WshEventData {
        req_id: Some(1),
        data_json: Some(r#"{"event":"earnings"}"#.into()),
    };

    let result = decode_wsh_event_data_proto(proto_msg).unwrap();
    assert_eq!(result.data_json, r#"{"event":"earnings"}"#);
}

#[test]
fn decode_wsh_metadata_rejects_text_framing() {
    use crate::common::request_helpers::OneShot;
    use crate::subscriptions::{DecoderContext, StreamDecoder};
    use crate::wsh::common::stream_decoders::WshMetadataRequest;

    // Text-framed arrival at a proto-only decoder must surface
    // UnexpectedWireFormat (docs/rules/wire/proto-only-decoding.md).
    assert_rejects_text_framing(IncomingMessages::WshMetaData, "104\09000\0{\"hi\":1}\0", |message| {
        OneShot::<WshMetadataRequest>::decode(&DecoderContext::default(), message).map(OneShot::into_inner)
    });
}

#[test]
fn decode_wsh_event_data_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::WshEventData, "105\09000\0{\"event\":\"e\"}\0", decode_wsh_event_data);
}
