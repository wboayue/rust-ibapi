use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use time::macros::format_description;
use time::OffsetDateTime;

use super::{RequestMessage, ResponseMessage};

static RECORDING_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
pub(crate) struct MessageRecorder {
    enabled: bool,
    recording_dir: String,
}

impl MessageRecorder {
    pub fn new() -> Self {
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
                    let recording_dir = format!("{}/{}", dir, now.format(&format).unwrap());

                    fs::create_dir_all(&recording_dir).unwrap();

                    MessageRecorder {
                        enabled: true,
                        recording_dir,
                    }
                }
            }
            _ => MessageRecorder {
                enabled: false,
                recording_dir: String::from(""),
            },
        }
    }

    pub fn record_request(&self, message: &RequestMessage) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        fs::write(self.request_file(record_id), message.encode().replace('\0', "|")).unwrap();
    }

    pub fn record_response(&self, message: &ResponseMessage) {
        if !self.enabled {
            return;
        }

        let record_id = RECORDING_SEQ.fetch_add(1, Ordering::SeqCst);
        fs::write(self.response_file(record_id), message.encode().replace('\0', "|")).unwrap();
    }

    fn request_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-request.msg", self.recording_dir, record_id)
    }

    fn response_file(&self, record_id: usize) -> String {
        format!("{}/{:04}-response.msg", self.recording_dir, record_id)
    }
}
