use super::fold_one_shot;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

fn current_time_message() -> ResponseMessage {
    ResponseMessage::from_protobuf(IncomingMessages::CurrentTime as i32, vec![])
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
