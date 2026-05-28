use super::*;

#[test]
fn test_decode_wsh_metadata_proto() {
    let bytes = crate::proto::WshMetaData {
        req_id: Some(1),
        data_json: Some(r#"{"key":"value"}"#.into()),
    }
    .encode_to_vec();

    let result = decode_wsh_metadata_proto(&bytes).unwrap();
    assert_eq!(result.data_json, r#"{"key":"value"}"#);
}

#[test]
fn test_decode_wsh_event_data_proto() {
    let bytes = crate::proto::WshEventData {
        req_id: Some(1),
        data_json: Some(r#"{"event":"earnings"}"#.into()),
    }
    .encode_to_vec();

    let result = decode_wsh_event_data_proto(&bytes).unwrap();
    assert_eq!(result.data_json, r#"{"event":"earnings"}"#);
}

#[test]
fn test_decode_wsh_metadata_rejects_text_framing() {
    // Text-framed arrival at a proto-only decoder must surface
    // UnexpectedResponse (rule 20).
    let message = ResponseMessage::from("104\09000\0{\"hi\":1}\0");
    match decode_wsh_metadata(&message) {
        Err(Error::UnexpectedResponse(_)) => {}
        other => panic!("expected UnexpectedResponse, got {other:?}"),
    }
}

#[test]
fn test_decode_wsh_event_data_rejects_text_framing() {
    let message = ResponseMessage::from("105\09000\0{\"event\":\"e\"}\0");
    match decode_wsh_event_data(&message) {
        Err(Error::UnexpectedResponse(_)) => {}
        other => panic!("expected UnexpectedResponse, got {other:?}"),
    }
}
