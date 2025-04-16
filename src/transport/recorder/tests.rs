use crate::messages::OutgoingMessages;
use crate::testdata::responses::{MANAGED_ACCOUNT, MARKET_RULE};

use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_message_recorder_new_with_empty_env_var() {
    temp_env::with_var("IBAPI_RECORDING_DIR", Some(""), || {
        let recorder = MessageRecorder::new();
        assert!(!recorder.enabled);
        assert_eq!(recorder.recording_dir, "");
    });
}

#[test]
fn test_message_recorder_new_with_valid_env_var() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let recorder = MessageRecorder::new();

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
        let mut message = RequestMessage::new();
        message.push_field(&OutgoingMessages::CancelAccountSummary);
        message.push_field(&9000);

        let recorder = MessageRecorder::new();
        recorder.record_request(&message);

        let files = fs::read_dir(&recorder.recording_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("-request.msg"));

        let content = fs::read_to_string(&files[0]).unwrap();
        assert_eq!(content, "63|9000|");
    });
}

#[test]
fn test_record_response() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let message = ResponseMessage::from_simple(MARKET_RULE);

        let recorder = MessageRecorder::new();
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
fn test_multiple_records() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    temp_env::with_var("IBAPI_RECORDING_DIR", Some(temp_path), || {
        let mut request = RequestMessage::new();
        request.push_field(&1);
        request.push_field(&"test_request");

        let response = ResponseMessage::from_simple(MANAGED_ACCOUNT);

        let recorder = MessageRecorder::new();

        recorder.record_request(&request);
        recorder.record_response(&response);

        let files = fs::read_dir(&recorder.recording_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        assert_eq!(files.len(), 2);

        let mut request_file = None;
        let mut response_file = None;

        for file in files {
            let file_name = file.file_name().unwrap().to_str().unwrap();
            if file_name.ends_with("-request.msg") {
                request_file = Some(file);
            } else if file_name.ends_with("-response.msg") {
                response_file = Some(file);
            }
        }

        assert!(request_file.is_some());
        assert!(response_file.is_some());

        let request_content = fs::read_to_string(request_file.unwrap()).unwrap();
        let response_content = fs::read_to_string(response_file.unwrap()).unwrap();

        assert_eq!(request_content, "1|test_request|");
        assert_eq!(response_content, "15|1|DU1234567,DU7654321|");
    });
}

#[test]
fn test_disabled_recorder() {
    temp_env::with_var("IBAPI_RECORDING_DIR", Some(""), || {
        let recorder = MessageRecorder::new();
        assert!(!recorder.enabled);

        let request = RequestMessage::new();
        let response = ResponseMessage::from_simple(MANAGED_ACCOUNT);

        // These should not panic or create any files
        recorder.record_request(&request);
        recorder.record_response(&response);
    });
}
