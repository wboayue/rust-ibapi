//! The MessageRecorder is used to log interactions between the client and
//! the TWS server.
//! The record is enabled by setting the environment variable IBAPI_RECORDING_DIR
//! IBAPI_RECORDING_DIR is set to the path to store logs
//! e.g.  set to /tmp/logs
//! /tmp/logs/0001-request.msg
//! /tmp/logs/0002-response.msg

use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::warn;

use time::macros::format_description;
use time::OffsetDateTime;

use super::ResponseMessage;

static RECORDING_SEQ: AtomicUsize = AtomicUsize::new(0);
static RECORDER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
pub(crate) struct MessageRecorder {
    enabled: bool,
    recording_dir: String,
}

impl MessageRecorder {
    pub fn new(enabled: bool, recording_dir: String) -> Self {
        Self { enabled, recording_dir }
    }
    pub fn from_env() -> Self {
        match env::var("IBAPI_RECORDING_DIR") {
            Ok(dir) => {
                if dir.is_empty() {
                    MessageRecorder {
                        enabled: false,
                        recording_dir: String::from(""),
                    }
                } else {
                    let format = format_description!("[year]-[month]-[day]-[hour]-[minute]");
                    let now = OffsetDateTime::now_utc();
                    let instance_id = RECORDER_ID.fetch_add(1, Ordering::SeqCst);
                    let recording_dir = format!("{}/{}-{}", dir, now.format(&format).unwrap(), instance_id);

                    fs::create_dir_all(&recording_dir).unwrap();

                    MessageRecorder::new(true, recording_dir)
                }
            }
            _ => MessageRecorder {
                enabled: false,
                recording_dir: String::from(""),
            },
        }
    }

    pub fn record_request(&self, data: &[u8]) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        if let Err(err) = fs::write(self.request_file(record_id), data) {
            warn!("failed to record request: {err}");
        }
    }

    pub fn record_response(&self, message: &ResponseMessage) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        if let Err(err) = fs::write(self.response_file(record_id), message.encode().replace('\0', "|")) {
            warn!("failed to record response: {err}");
        }
    }

    fn request_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-request.msg", self.recording_dir, record_id)
    }

    fn response_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-response.msg", self.recording_dir, record_id)
    }
}

#[cfg(test)]
mod tests {
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
}
