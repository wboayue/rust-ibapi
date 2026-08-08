use super::*;
use crate::common::test_utils::helpers::assert_rejects_text_framing;
use crate::messages::IncomingMessages;

#[test]
fn test_decode_wsh_metadata_proto() {
    let bytes = crate::proto::WshMetaData {
        req_id: Some(1),
        data_json: Some(r#"{"key":"value"}"#.into()),
    }
    .encode_to_vec();

    let result = decode_wsh_metadata_proto(prost::Message::decode(&bytes[..]).expect("fixture must decode")).unwrap();
    assert_eq!(result.data_json, r#"{"key":"value"}"#);
}

#[test]
fn test_decode_wsh_event_data_proto() {
    let bytes = crate::proto::WshEventData {
        req_id: Some(1),
        data_json: Some(r#"{"event":"earnings"}"#.into()),
    }
    .encode_to_vec();

    let result = decode_wsh_event_data_proto(prost::Message::decode(&bytes[..]).expect("fixture must decode")).unwrap();
    assert_eq!(result.data_json, r#"{"event":"earnings"}"#);
}

#[test]
fn test_decode_wsh_metadata_rejects_text_framing() {
    // Text-framed arrival at a proto-only decoder must surface
    // UnexpectedWireFormat (docs/rules/wire/proto-only-decoding.md).
    assert_rejects_text_framing(IncomingMessages::WshMetaData, "104\09000\0{\"hi\":1}\0", decode_wsh_metadata);
}

#[test]
fn test_decode_wsh_event_data_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::WshEventData, "105\09000\0{\"event\":\"e\"}\0", decode_wsh_event_data);
}
