use super::{expect_proto, fold_one_shot, fold_one_shot_mut};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

fn current_time_message() -> ResponseMessage {
    crate::common::test_utils::helpers::proto_response(IncomingMessages::CurrentTime, vec![])
}

#[test]
fn test_fold_one_shot_processes_response() {
    let result = fold_one_shot(
        Some(Ok(current_time_message())),
        |message| Ok(message.message_type()),
        || Err(Error::UnexpectedEndOfStream),
    );
    assert!(matches!(result, Ok(IncomingMessages::CurrentTime)));
}

#[test]
fn test_fold_one_shot_propagates_error() {
    // A routed error (e.g. request-less hard error fanned to one-shot shared
    // channels, #694) must surface to the caller, never be masked by on_none.
    let result: Result<IncomingMessages, Error> = fold_one_shot(
        Some(Err(Error::UnexpectedEndOfStream)),
        |message| Ok(message.message_type()),
        || panic!("on_none must not run for Some(Err)"),
    );
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)));
}

#[test]
fn test_fold_one_shot_delegates_closed_stream_to_on_none() {
    // on_none decides what a closed stream means: a default value...
    let result = fold_one_shot(None, |_| Ok(1), || Ok(0));
    assert!(matches!(result, Ok(0)));

    // ...or an error.
    let result: Result<i32, Error> = fold_one_shot(None, |_| Ok(1), || Err(Error::UnexpectedEndOfStream));
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)));
}

#[test]
fn test_fold_one_shot_mut_hands_the_processor_a_mutable_borrow() {
    // The three arms are covered above — `fold_one_shot` delegates here — so all
    // this pins is the `&mut` shape the four option-computation sites need.
    let result = fold_one_shot_mut(
        Some(Ok(current_time_message())),
        |m: &mut ResponseMessage| Ok(m.message_type()),
        || Err(Error::UnexpectedEndOfStream),
    );
    assert!(matches!(result, Ok(IncomingMessages::CurrentTime)));
}

#[test]
fn test_expect_proto_decodes_the_expected_type() {
    let processor = expect_proto(IncomingMessages::CurrentTime, |bytes: &[u8]| Ok(bytes.len()));
    assert!(matches!(processor(&current_time_message()), Ok(0)));
}

#[test]
fn test_expect_proto_rejects_a_foreign_type() {
    // The frame is well-formed proto — only the type is wrong. Without the
    // narrow, the payload decoder would run on another message's bytes.
    let processor = expect_proto(IncomingMessages::UserInfo, |bytes: &[u8]| Ok(bytes.len()));
    let err = processor(&current_time_message()).expect_err("foreign type must be rejected");
    assert!(matches!(err, Error::UnexpectedResponse(_)), "got {err:?}");
}

#[test]
fn test_expect_proto_rejects_text_framing_of_the_expected_type() {
    // Right type, unreadable framing: UnexpectedWireFormat, not UnexpectedResponse (#731).
    let message = ResponseMessage::from("49\01\01678890000\0");
    let processor = expect_proto(IncomingMessages::CurrentTime, |bytes: &[u8]| Ok(bytes.len()));
    let err = processor(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got {err:?}");
}
