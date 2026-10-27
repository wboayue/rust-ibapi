use super::{empty_on_end_of_stream, expect_proto, fold_one_shot};
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::Error;

fn current_time_message() -> ResponseMessage {
    crate::common::test_utils::helpers::proto_response(IncomingMessages::CurrentTime, vec![])
}

#[test]
fn fold_one_shot_processes_response() {
    let result = fold_one_shot(Some(Ok(current_time_message())), |message| Ok(message.message_type()));
    assert!(matches!(result, Ok(IncomingMessages::CurrentTime)));
}

#[test]
fn fold_one_shot_propagates_error() {
    // A routed error (e.g. request-less hard error fanned to one-shot shared
    // channels, #694) must surface to the caller, never be masked.
    let result: Result<IncomingMessages, Error> = fold_one_shot(Some(Err(Error::Cancelled)), |message| Ok(message.message_type()));
    assert!(matches!(result, Err(Error::Cancelled)));
}

#[test]
fn fold_one_shot_reads_a_closed_stream_as_an_error() {
    let result: Result<i32, Error> = fold_one_shot(None, |_| Ok(1));
    assert!(matches!(result, Err(Error::UnexpectedEndOfStream)));
}

#[test]
fn empty_on_end_of_stream_converts_only_that_variant() {
    // The ten collection sites read a closed stream as "nothing to report"...
    let recovered: Result<Vec<i32>, Error> = empty_on_end_of_stream(Error::UnexpectedEndOfStream);
    assert_eq!(recovered.expect("end of stream becomes the empty collection"), Vec::<i32>::new());

    // ...without swallowing a real failure that happened to arrive first.
    let passed_through: Result<Vec<i32>, Error> = empty_on_end_of_stream(Error::Cancelled);
    assert!(matches!(passed_through, Err(Error::Cancelled)));
}

#[test]
fn expect_proto_decodes_the_expected_type() {
    // The expected type is not passed: it is CurrentTime::MESSAGE_ID, reached
    // through the payload this decoder accepts.
    let processor = expect_proto(|payload: crate::proto::CurrentTime| Ok(payload.current_time));
    assert!(matches!(processor(&current_time_message()), Ok(None)));
}

#[test]
fn expect_proto_rejects_a_foreign_type() {
    // The frame is well-formed proto — only the type is wrong. Without the
    // narrow, the payload decoder would run on another message's bytes.
    let processor = expect_proto(|payload: crate::proto::UserInfo| Ok(payload.white_branding_id));
    let err = processor(&current_time_message()).expect_err("foreign type must be rejected");
    assert!(matches!(err, Error::UnexpectedResponse(_)), "got {err:?}");
}

#[test]
fn expect_proto_narrows_to_the_payloads_own_message_id() {
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
    fn sync_one_shot_resends_after_a_connection_reset() {
        use crate::client::blocking::Client;

        let message_bus = stub(2);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        client.server_time().expect("two resets are under the retry limit");
        assert_eq!(message_bus.request_messages().len(), 3, "each retry must re-send the request");
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_one_shot_gives_up_past_the_retry_limit() {
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
    async fn async_one_shot_resends_after_a_connection_reset() {
        use crate::Client;

        let message_bus = stub(2);
        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        client.server_time().await.expect("two resets are under the retry limit");
        assert_eq!(message_bus.request_messages().len(), 3, "each retry must re-send the request");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_one_shot_gives_up_past_the_retry_limit() {
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

#[cfg(test)]
mod bounded_wait {
    use std::sync::Arc;
    use std::time::Duration;

    use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
    use crate::stubs::MessageBusStub;
    use crate::testdata::builders::accounts::current_time;
    use crate::testdata::builders::ResponseProtoEncoder;
    use crate::{common::test_utils::helpers::proto_response, server_versions, Error};

    const SHORT: Duration = Duration::from_millis(20);

    fn frame() -> ResponseMessage {
        proto_response(IncomingMessages::CurrentTime, current_time().encode_proto())
    }

    /// A stub whose channels never close: `frames` then silence, as TWS not answering.
    fn silent_stub(frames: usize) -> Arc<MessageBusStub> {
        Arc::new(MessageBusStub::with_ordered_responses(vec![frame(); frames]).keeping_channels_open())
    }

    fn encoder() -> Result<Vec<u8>, Error> {
        Ok(vec![])
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_one_shot_shared_times_out_without_a_frame() {
        use crate::client::blocking::Client;
        use crate::common::request_helpers::blocking::one_shot_shared_within;

        let client = Client::stubbed(silent_stub(0), server_versions::SIZE_RULES);
        let error = one_shot_shared_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, |_| Ok(())).unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_one_shot_shared_matching_reads_past_a_foreign_frame() {
        use crate::client::blocking::Client;
        use crate::common::request_helpers::blocking::one_shot_shared_matching_within;
        use std::cell::Cell;

        let client = Client::stubbed(silent_stub(2), server_versions::SIZE_RULES);
        let seen = Cell::new(0);
        let processor = |_: &ResponseMessage| {
            seen.set(seen.get() + 1);
            Ok((seen.get() == 2).then_some("second"))
        };
        let answer = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, processor).unwrap();
        assert_eq!(answer, "second");
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_one_shot_shared_matching_times_out_on_foreign_frames_only() {
        use crate::client::blocking::Client;
        use crate::common::request_helpers::blocking::one_shot_shared_matching_within;

        let client = Client::stubbed(silent_stub(1), server_versions::SIZE_RULES);
        let error = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, |_| Ok(None::<()>)).unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_one_shot_shared_matching_stops_reading_at_the_deadline() {
        use crate::client::blocking::Client;
        use crate::common::request_helpers::blocking::one_shot_shared_matching_within;
        use std::cell::Cell;

        // Foreign frames already queued: a passed deadline must not read them.
        let client = Client::stubbed(silent_stub(3), server_versions::SIZE_RULES);
        let seen = Cell::new(0);
        let processor = |_: &ResponseMessage| {
            seen.set(seen.get() + 1);
            Ok(None::<()>)
        };
        let error = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, Duration::ZERO, encoder, processor).unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
        assert_eq!(seen.get(), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_one_shot_shared_times_out_without_a_frame() {
        use crate::common::request_helpers::one_shot_shared_within;
        use crate::Client;

        let client = Client::stubbed(silent_stub(0), server_versions::SIZE_RULES);
        let error = one_shot_shared_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, |_| Ok(()))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_one_shot_shared_matching_reads_past_a_foreign_frame() {
        use crate::common::request_helpers::one_shot_shared_matching_within;
        use crate::Client;
        use std::cell::Cell;

        let client = Client::stubbed(silent_stub(2), server_versions::SIZE_RULES);
        let seen = Cell::new(0);
        let processor = |_: &ResponseMessage| {
            seen.set(seen.get() + 1);
            Ok((seen.get() == 2).then_some("second"))
        };
        let answer = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, processor)
            .await
            .unwrap();
        assert_eq!(answer, "second");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_one_shot_shared_matching_times_out_on_foreign_frames_only() {
        use crate::common::request_helpers::one_shot_shared_matching_within;
        use crate::Client;

        let client = Client::stubbed(silent_stub(1), server_versions::SIZE_RULES);
        let error = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, SHORT, encoder, |_| Ok(None::<()>))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_one_shot_shared_matching_stops_reading_at_the_deadline() {
        use crate::common::request_helpers::one_shot_shared_matching_within;
        use crate::Client;
        use std::cell::Cell;

        // Foreign frames already queued: a passed deadline must not read them.
        let client = Client::stubbed(silent_stub(3), server_versions::SIZE_RULES);
        let seen = Cell::new(0);
        let processor = |_: &ResponseMessage| {
            seen.set(seen.get() + 1);
            Ok(None::<()>)
        };
        let error = one_shot_shared_matching_within(&client, OutgoingMessages::RequestCurrentTime, Duration::ZERO, encoder, processor)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Timeout), "got {error:?}");
        assert_eq!(seen.get(), 0);
    }
}

#[test]
fn expect_proto_rejects_text_framing_of_the_expected_type() {
    // Right type, unreadable framing: UnexpectedWireFormat, not UnexpectedResponse (#731).
    let message = ResponseMessage::from("49\01\01678890000\0");
    let processor = expect_proto(|payload: crate::proto::CurrentTime| Ok(payload.current_time));
    let err = processor(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got {err:?}");
}

/// `one_shot_cancelling` (#1107): cancelled at TWS unless the request ended
/// there, by its answer or a TWS error.
mod cancelling {
    use std::sync::Arc;

    use super::super::CancellableOneShot;
    use crate::common::test_utils::helpers::{count_proto_msgs, proto_error_response, proto_response, TEST_REQ_ID_FIRST};
    use crate::contracts::Contract;
    use crate::market_data::historical::{BarSize, Duration, WhatToShow};
    use crate::market_data::TradingHours;
    use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
    use crate::stubs::MessageBusStub;
    use crate::{server_versions, Error};

    const REQUEST: &[u8] = b"request";
    const CANCEL: &[u8] = b"cancel";

    /// Answered by a `HeadTimestamp` frame (any request-id-routed payload
    /// does; the debug routing guard rejects one that isn't).
    enum Answered {}

    impl CancellableOneShot for Answered {
        type Proto = crate::proto::HeadTimestamp;
        type Output = ();

        fn decode(_payload: Self::Proto) -> Result<(), Error> {
            Ok(())
        }

        fn cancel(_request_id: i32) -> Result<Vec<u8>, Error> {
            Ok(CANCEL.to_vec())
        }
    }

    /// Its answer fails to decode.
    enum Undecodable {}

    impl CancellableOneShot for Undecodable {
        type Proto = crate::proto::HeadTimestamp;
        type Output = ();

        fn decode(_payload: Self::Proto) -> Result<(), Error> {
            Err(Error::Simple("bad answer".into()))
        }

        fn cancel(_request_id: i32) -> Result<Vec<u8>, Error> {
            Ok(CANCEL.to_vec())
        }
    }

    fn answer() -> ResponseMessage {
        proto_response(IncomingMessages::HeadTimestamp, vec![])
    }

    fn tws_error() -> ResponseMessage {
        proto_error_response(TEST_REQ_ID_FIRST, 200, "No security definition has been found for the request")
    }

    fn cancels(bus: &MessageBusStub) -> usize {
        bus.request_messages().iter().filter(|m| m.as_slice() == CANCEL).count()
    }

    /// (label, responses, decodes, expect Ok, cancels written)
    fn cases() -> Vec<(&'static str, Vec<ResponseMessage>, bool, bool, usize)> {
        vec![
            ("answered", vec![answer()], true, true, 0),
            ("TWS error ends the request", vec![tws_error()], true, false, 0),
            ("closed before the answer", vec![], true, false, 1),
            ("answer fails to decode", vec![answer()], false, false, 1),
        ]
    }

    #[cfg(feature = "sync")]
    #[test]
    fn sync_cancels_unless_ended_at_tws() {
        use crate::client::blocking::Client;
        use crate::common::request_helpers::blocking::one_shot_cancelling;

        for (label, responses, decodes, ok, expected_cancels) in cases() {
            let bus = Arc::new(MessageBusStub::with_ordered_responses(responses));
            let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
            let encoder = |_| Ok(REQUEST.to_vec());

            let result = if decodes {
                one_shot_cancelling::<Answered>(&client, encoder)
            } else {
                one_shot_cancelling::<Undecodable>(&client, encoder)
            };

            assert_eq!(result.is_ok(), ok, "{label}: {result:?}");
            assert_eq!(bus.request_messages()[0], REQUEST, "{label}");
            assert_eq!(cancels(&bus), expected_cancels, "{label}");
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_cancels_unless_ended_at_tws() {
        use crate::common::request_helpers::one_shot_cancelling;
        use crate::Client;

        for (label, responses, decodes, ok, expected_cancels) in cases() {
            let bus = Arc::new(MessageBusStub::with_ordered_responses(responses));
            let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
            let encoder = |_| Ok(REQUEST.to_vec());

            let result = if decodes {
                one_shot_cancelling::<Answered>(&client, encoder).await
            } else {
                one_shot_cancelling::<Undecodable>(&client, encoder).await
            };
            // Drop spawns its cancel; let it run.
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            assert_eq!(result.is_ok(), ok, "{label}: {result:?}");
            assert_eq!(bus.request_messages()[0], REQUEST, "{label}");
            assert_eq!(cancels(&bus), expected_cancels, "{label}");
        }
    }

    /// The case the issue is about: a caller's timeout drops the pending
    /// future, and the request is cancelled at TWS.
    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_timeout_cancels() {
        use crate::common::request_helpers::one_shot_cancelling;
        use crate::Client;

        let bus = Arc::new(MessageBusStub::default().keeping_channels_open());
        let client = Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);

        let pending = one_shot_cancelling::<Answered>(&client, |_| Ok(REQUEST.to_vec()));
        let result = tokio::time::timeout(std::time::Duration::from_millis(20), pending).await;
        assert!(result.is_err(), "no answer, so the wait times out");

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
        while cancels(&bus) == 0 && tokio::time::Instant::now() < deadline {
            tokio::task::yield_now().await;
        }
        assert_eq!(cancels(&bus), 1);
    }

    fn stock() -> Contract {
        Contract::stock("AAPL").build()
    }

    fn option() -> Contract {
        Contract::call("AAPL").strike(240.0).expires_on(2026, 11, 20).build()
    }

    /// Every cancellable one-shot goes through `one_shot_cancelling` with its
    /// own cancel: unanswered, each sends the cancel IB defines for it.
    #[cfg(feature = "sync")]
    #[test]
    fn sync_requests_send_their_cancel() {
        use crate::client::blocking::Client;

        type Call = fn(&Client);
        let requests: [(&str, Call, OutgoingMessages); 7] = [
            (
                "head_timestamp",
                |c| drop(c.head_timestamp(&stock(), WhatToShow::Trades, TradingHours::Regular)),
                OutgoingMessages::CancelHeadTimestamp,
            ),
            (
                "histogram_data",
                |c| drop(c.histogram_data(&stock(), TradingHours::Regular, BarSize::Week)),
                OutgoingMessages::CancelHistogramData,
            ),
            (
                "historical_schedules",
                |c| drop(c.historical_schedules(&stock(), Duration::days(1)).fetch()),
                OutgoingMessages::CancelHistoricalData,
            ),
            (
                "calculate_option_price",
                |c| drop(c.calculate_option_price(&option(), 0.3, 235.0)),
                OutgoingMessages::CancelOptionPrice,
            ),
            (
                "calculate_implied_volatility",
                |c| drop(c.calculate_implied_volatility(&option(), 10.0, 235.0)),
                OutgoingMessages::CancelImpliedVolatility,
            ),
            ("wsh_metadata", |c| drop(c.wsh_metadata()), OutgoingMessages::CancelWshMetaData),
            (
                "wsh_event_data",
                |c| drop(c.wsh_event_data_by_contract(1).fetch()),
                OutgoingMessages::CancelWshEventData,
            ),
        ];

        for (label, call, cancel) in requests {
            let bus = Arc::new(MessageBusStub::default());
            call(&Client::stubbed(bus.clone(), server_versions::CONDITIONS_INCLUDE_OVERNIGHT));
            assert_eq!(count_proto_msgs(&bus.request_messages(), cancel), 1, "{label}");
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_requests_send_their_cancel() {
        use crate::Client;

        let bus = || Arc::new(MessageBusStub::default());
        let client = |bus: &Arc<MessageBusStub>| Client::stubbed(bus.clone(), server_versions::CONDITIONS_INCLUDE_OVERNIGHT);
        let mut runs = Vec::new();

        let b = bus();
        drop(client(&b).head_timestamp(&stock(), WhatToShow::Trades, TradingHours::Regular).await);
        runs.push(("head_timestamp", b, OutgoingMessages::CancelHeadTimestamp));
        let b = bus();
        drop(client(&b).histogram_data(&stock(), TradingHours::Regular, BarSize::Week).await);
        runs.push(("histogram_data", b, OutgoingMessages::CancelHistogramData));
        let b = bus();
        drop(client(&b).historical_schedules(&stock(), Duration::days(1)).fetch().await);
        runs.push(("historical_schedules", b, OutgoingMessages::CancelHistoricalData));
        let b = bus();
        drop(client(&b).calculate_option_price(&option(), 0.3, 235.0).await);
        runs.push(("calculate_option_price", b, OutgoingMessages::CancelOptionPrice));
        let b = bus();
        drop(client(&b).calculate_implied_volatility(&option(), 10.0, 235.0).await);
        runs.push(("calculate_implied_volatility", b, OutgoingMessages::CancelImpliedVolatility));
        let b = bus();
        drop(client(&b).wsh_metadata().await);
        runs.push(("wsh_metadata", b, OutgoingMessages::CancelWshMetaData));
        let b = bus();
        drop(client(&b).wsh_event_data_by_contract(1).fetch().await);
        runs.push(("wsh_event_data", b, OutgoingMessages::CancelWshEventData));

        // Drops spawn their cancels; let them run.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        for (label, bus, cancel) in runs {
            assert_eq!(count_proto_msgs(&bus.request_messages(), cancel), 1, "{label}");
        }
    }
}
