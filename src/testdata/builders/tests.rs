use super::positions::{position, position_end};
use super::{response_messages, ResponseEncoder};

struct DummyMessage;

impl ResponseEncoder for DummyMessage {
    fn fields(&self) -> Vec<String> {
        vec!["9".to_string(), "1".to_string(), "hello".to_string()]
    }
}

#[test]
fn encode_pipe_default_joins_with_pipe_and_trailing_separator() {
    assert_eq!(DummyMessage.encode_pipe(), "9|1|hello|");
}

#[test]
fn encode_null_default_joins_with_nul_and_trailing_separator() {
    assert_eq!(DummyMessage.encode_null(), "9\01\0hello\0");
}

#[test]
fn encode_length_prefixed_default_wraps_null_payload_with_be_length() {
    let bytes = DummyMessage.encode_length_prefixed();
    let payload_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    assert_eq!(payload_len, bytes.len() - 4);
    assert_eq!(&bytes[4..], DummyMessage.encode_null().as_bytes());
}

#[test]
fn response_messages_collects_heterogeneous_builders() {
    let msgs = response_messages(&[&position(), &position_end()]);

    assert_eq!(msgs.len(), 2);
    assert!(msgs[0].starts_with("61|3|"));
    assert_eq!(msgs[1], "62|1|");
}

#[test]
fn response_messages_empty_input_yields_empty_vec() {
    let msgs = response_messages(&[]);
    assert!(msgs.is_empty());
}
