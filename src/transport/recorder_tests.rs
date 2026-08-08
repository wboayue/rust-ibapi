use crate::messages::encode_protobuf_message;
use crate::testdata::responses::{MANAGED_ACCOUNT, MARKET_RULE};

use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_message_recorder_new_with_empty_env_var() {
    temp_env::with_var("IBAPI_RECORDING_DIR", Some(""), || {
        let recorder = MessageRecorder::from_env();
        assert!(!recorder.enabled);
        assert_eq!(recorder.recording_dir, "");
    });
}

#[test]
fn test_message_recorder_new_with_valid_env_var() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let recorder = MessageRecorder::from_env();

        assert!(recorder.enabled);
        assert!(recorder.recording_dir.starts_with(temp_path));
        assert!(fs::metadata(&recorder.recording_dir).unwrap().is_dir());
    });
}

#[test]
fn test_record_request() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let data = encode_protobuf_message(63, &[0x08, 0xd0, 0x46]); // msg_id=63, proto payload

        let recorder = MessageRecorder::from_env();
        recorder.record_request(&data);

        let files = fs::read_dir(&recorder.recording_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("-request.msg"));

        let content = fs::read(&files[0]).unwrap();
        assert_eq!(content, data);
    });
}

#[test]
fn test_record_response() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let message = ResponseMessage::from_simple(MARKET_RULE);

        let recorder = MessageRecorder::from_env();
        recorder.record_response(&message);

        let files = fs::read_dir(&recorder.recording_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("-response.msg"));

        let content = fs::read_to_string(&files[0]).unwrap();
        assert_eq!(content, message.encode_simple());
    });
}

#[test]
fn test_record_response_writes_the_protobuf_frame() {
    // Every response at the protocol floor is proto-framed, and this used to
    // record `fields.join("\0")` — which for a proto message is the bare
    // message id, losing the entire payload.
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let payload = vec![0x08, 0xd0, 0x46];
        let message = ResponseMessage::from_protobuf(crate::messages::IncomingMessages::CurrentTime as i32, payload.clone());

        let recorder = MessageRecorder::from_env();
        recorder.record_response(&message);

        let file = fs::read_dir(&recorder.recording_dir).unwrap().next().unwrap().unwrap().path();
        let content = fs::read(&file).unwrap();

        assert_eq!(
            content,
            encode_protobuf_message(crate::messages::IncomingMessages::CurrentTime as i32, &payload),
            "a recorded response must be the wire frame a replay would read"
        );
        assert!(content.ends_with(&payload), "the payload must survive the round trip");
    });
}

#[test]
fn test_multiple_records() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let request_data = encode_protobuf_message(1, &[]);
        let response = ResponseMessage::from_simple(MANAGED_ACCOUNT);

        let recorder = MessageRecorder::from_env();

        recorder.record_request(&request_data);
        recorder.record_response(&response);

        let files = fs::read_dir(&recorder.recording_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        assert_eq!(files.len(), 2);
    });
}

#[test]
fn test_disabled_recorder() {
    temp_env::with_var("IBAPI_RECORDING_DIR", Some(""), || {
        let recorder = MessageRecorder::from_env();
        assert!(!recorder.enabled);

        let response = ResponseMessage::from_simple(MANAGED_ACCOUNT);

        recorder.record_request(&[]);
        recorder.record_response(&response);
    });
}
