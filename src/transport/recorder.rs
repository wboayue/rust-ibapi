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
#[path = "recorder_tests.rs"]
mod tests;
