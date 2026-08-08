use super::{expect_proto, fold_one_shot};
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

/// The retry wiring, exercised through a public one-shot rather than through
/// the combinator.
///
/// `src/common/retry.rs` covers `retry_on_connection_reset` itself, but nothing
/// covered the wiring — that a one-shot re-encodes and re-sends its request when
/// the transport hands it a reset. That gap is why #741 could move ten APIs onto
/// the retrying helper on the strength of reading the call site. `server_time`
/// stands in for the 54 sites; the resend count is `request_messages()`.
#[cfg(test)]
mod retry_wiring {
    use crate::messages::IncomingMessages;
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::accounts::current_time;
    use crate::testdata::builders::ResponseProtoEncoder;
    use crate::{common::test_utils::helpers::proto_response, server_versions, Error};
    use std::sync::Arc;

    fn stub(resets: usize) -> Arc<MessageBusStub> {
        Arc::new(
            MessageBusStub::with_ordered_responses(vec![proto_response(IncomingMessages::CurrentTime, current_time().encode_proto())])
                .with_connection_resets(resets),
        )
    }

    #[cfg(feature = "sync")]
    #[test]
    fn test_sync_one_shot_resends_after_a_connection_reset() {
        use crate::client::blocking::Client;

        let message_bus = stub(2);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        client.server_time().expect("two resets are under the retry limit");
        assert_eq!(message_bus.request_messages().len(), 3, "each retry must re-send the request");
    }

    #[cfg(feature = "sync")]
    #[test]
    fn test_sync_one_shot_gives_up_past_the_retry_limit() {
        use crate::client::blocking::Client;
        use crate::common::retry::DEFAULT_MAX_RETRIES;

        let attempts = DEFAULT_MAX_RETRIES as usize + 1;
        let message_bus = stub(attempts);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let error = client.server_time().expect_err("the reset must surface once retries are spent");
        assert!(matches!(error, Error::ConnectionReset), "got {error:?}");
        assert_eq!(message_bus.request_messages().len(), attempts);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_one_shot_resends_after_a_connection_reset() {
        use crate::Client;

        let message_bus = stub(2);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        client.server_time().await.expect("two resets are under the retry limit");
        assert_eq!(message_bus.request_messages().len(), 3, "each retry must re-send the request");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_one_shot_gives_up_past_the_retry_limit() {
        use crate::common::retry::DEFAULT_MAX_RETRIES;
        use crate::Client;

        let attempts = DEFAULT_MAX_RETRIES as usize + 1;
        let message_bus = stub(attempts);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let error = client.server_time().await.expect_err("the reset must surface once retries are spent");
        assert!(matches!(error, Error::ConnectionReset), "got {error:?}");
        assert_eq!(message_bus.request_messages().len(), attempts);
    }
}

#[test]
fn test_expect_proto_rejects_text_framing_of_the_expected_type() {
    // Right type, unreadable framing: UnexpectedWireFormat, not UnexpectedResponse (#731).
    let message = ResponseMessage::from("49\01\01678890000\0");
    let processor = expect_proto(IncomingMessages::CurrentTime, |bytes: &[u8]| Ok(bytes.len()));
    let err = processor(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got {err:?}");
}
