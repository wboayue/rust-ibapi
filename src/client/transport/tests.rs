use super::*;
use std::env;

#[test]
fn env_var_enables_recorder() {
    let key = String::from("IBAPI_RECORDING_DIR");
    let dir = String::from("/tmp/records");

    env::set_var(&key, &dir);

    let recorder = MessageRecorder::new();

    assert_eq!(true, recorder.enabled);
    assert!(&recorder.recording_dir.starts_with(&dir), "{} != {}", &recorder.recording_dir, &dir)
}

#[test]
fn recorder_is_disabled() {
    let key = String::from("IBAPI_RECORDING_DIR");

    env::set_var(&key, &"");

    let recorder = MessageRecorder::new();

    assert_eq!(false, recorder.enabled);
    assert_eq!("", &recorder.recording_dir);
}

#[test]
fn recorder_generates_output_file() {
    let recording_dir = String::from("/tmp/records");

    let recorder = MessageRecorder {
        enabled: true,
        recording_dir: recording_dir,
    };

    assert_eq!(format!("{}/0001-request.msg", recorder.recording_dir), recorder.request_file(1));
    assert_eq!(format!("{}/0002-response.msg", recorder.recording_dir), recorder.response_file(2));
}
