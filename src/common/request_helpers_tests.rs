use super::{empty_on_end_of_stream, expect_proto, fold_one_shot};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

fn current_time_message() -> ResponseMessage {
    crate::common::test_utils::helpers::proto_response(IncomingMessages::CurrentTime, vec![])
}

#[test]
fn test_fold_one_shot_processes_response() {
    let result = fold_one_shot(Some(Ok(current_time_message())), |message| Ok(message.message_type()));
    assert!(matches!(result, Ok(IncomingMessages::CurrentTime)));
}

#[test]
fn test_fold_one_shot_propagates_error() {
    // A routed error (e.g. request-less hard error fanned to one-shot shared
    // channels, #694) must surface to the caller, never be masked.
    let result: Result<IncomingMessages, Error> = fold_one_shot(Some(Err(Error::Cancelled)), |message| Ok(message.message_type()));
    assert!(matches!(result, Err(Error::Cancelled)));
}

#[test]
fn test_fold_one_shot_reads_a_closed_stream_as_an_error() {
    let result: Result<i32, Error> = fold_one_shot(None, |_| Ok(1));
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)));
}

#[test]
fn test_empty_on_end_of_stream_converts_only_that_variant() {
    // The ten collection sites read a closed stream as "nothing to report"...
    let recovered: Result<Vec<i32>, Error> = empty_on_end_of_stream(Error::UnexpectedEndOfStream);
    assert_eq!(recovered.expect("end of stream becomes the empty collection"), Vec::<i32>::new());

    // ...without swallowing a real failure that happened to arrive first.
    let passed_through: Result<Vec<i32>, Error> = empty_on_end_of_stream(Error::Cancelled);
    assert!(matches!(passed_through, Err(Error::Cancelled)));
}

#[test]
fn test_expect_proto_decodes_the_expected_type() {
    // The expected type is not passed: it is CurrentTime::MESSAGE_ID, reached
    // through the payload this decoder accepts.
    let processor = expect_proto(|payload: crate::proto::CurrentTime| Ok(payload.current_time));
    assert!(matches!(processor(&current_time_message()), Ok(None)));
}

#[test]
fn test_expect_proto_rejects_a_foreign_type() {
    // The frame is well-formed proto — only the type is wrong. Without the
    // narrow, the payload decoder would run on another message's bytes.
    let processor = expect_proto(|payload: crate::proto::UserInfo| Ok(payload.white_branding_id));
    let err = processor(&current_time_message()).expect_err("foreign type must be rejected");
    assert!(matches!(err, Error::UnexpectedResponse(_)), "got {err:?}");
}

#[test]
fn test_expect_proto_narrows_to_the_payloads_own_message_id() {
    // The pairing a hand-listed roster used to guard: every declared payload
    // narrows to its own MESSAGE_ID, and a frame of that type is what it
    // accepts. Checked here for one payload per collision-prone shape; the
    // compiler covers the rest, since a call site cannot name a type without
    // its id.
    use crate::proto::payload::ProtoPayload;

    assert_eq!(crate::proto::CurrentTime::MESSAGE_ID, IncomingMessages::CurrentTime);
    assert_eq!(crate::proto::CurrentTimeInMillis::MESSAGE_ID, IncomingMessages::CurrentTimeInMillis);
    assert_eq!(crate::proto::HeadTimestamp::MESSAGE_ID, IncomingMessages::HeadTimestamp);
    // The three whose proto name and wire name disagree.
    assert_eq!(crate::proto::MarketDepthExchanges::MESSAGE_ID, IncomingMessages::MktDepthExchanges);
    assert_eq!(crate::proto::SoftDollarTiers::MESSAGE_ID, IncomingMessages::SoftDollarTier);
    assert_eq!(crate::proto::ReceiveFa::MESSAGE_ID, IncomingMessages::ReceiveFA);
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
    let processor = expect_proto(|payload: crate::proto::CurrentTime| Ok(payload.current_time));
    let err = processor(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got {err:?}");
}
