use super::*;
use crate::common::test_utils::helpers::proto_response;
use crate::messages::ResponseMessage;
use crate::testdata::builders::contracts::{contract_data, contract_data_end, symbol_samples};
use crate::testdata::builders::ResponseProtoEncoder;

const ID: i32 = 42;

fn spec() -> RequestSpec {
    RequestSpec {
        data: IncomingMessages::ContractData,
        end: IncomingMessages::ContractDataEnd,
        limits: RawLimits {
            frames: 2,
            frame_bytes: 4096,
            total_bytes: 8192,
        },
    }
}

fn row() -> ResponseMessage {
    proto_response(IncomingMessages::ContractData, contract_data().request_id(ID).encode_proto())
}

fn end(id: i32) -> ResponseMessage {
    proto_response(IncomingMessages::ContractDataEnd, contract_data_end(id).encode_proto())
}

#[test]
fn retained_prefix_and_end_survive_cumulative_queue_limit() {
    let registry = Registry::default();
    let mut read = registry.register(ID, spec()).unwrap();
    let inbox = registry.get(ID).unwrap();
    inbox.push(row().into());
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Response(_)))));
    inbox.push(row().into());
    inbox.push(row().into()); // consumption did not replenish the frame budget
    inbox.push(end(ID).into());
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Response(_)))));
    assert!(matches!(
        read.take_next(),
        Some(Some(RoutedItem::Error(Error::ResponseLimitExceeded {
            resource: "frames",
            limit: 2
        })))
    ));
    assert!(matches!(read.take_next(), Some(None)));
    assert!(matches!(read.terminal(), Some(Terminal::End)));
}

#[test]
fn frame_and_total_bytes_stop_retention_but_do_not_forge_or_hide_end() {
    let size = row().raw_bytes.unwrap().len();
    for limits in [
        RawLimits {
            frame_bytes: size - 1,
            ..spec().limits
        },
        RawLimits {
            total_bytes: size,
            ..spec().limits
        },
    ] {
        let registry = Registry::default();
        let mut read = registry.register(ID, RequestSpec { limits, ..spec() }).unwrap();
        let inbox = registry.get(ID).unwrap();
        inbox.push(row().into());
        inbox.push(row().into());
        assert!(read.terminal().is_none());
        let journal = inbox.journal.lock().unwrap();
        assert!(journal.bytes <= limits.total_bytes);
        assert!(journal.queued.len() <= 1);
        assert!(matches!(journal.read_error, Some(Error::ResponseLimitExceeded { .. })));
        drop(journal);
        read.discard_buffered();
        assert!(matches!(read.take_next(), Some(None)));
        assert!(read.terminal().is_none());
        inbox.push(end(ID).into());
        assert!(matches!(read.terminal(), Some(Terminal::End)));
    }
}

#[test]
fn end_requires_correct_id_valid_protobuf_and_frame_budget() {
    for message in [
        end(ID + 1),
        proto_response(IncomingMessages::ContractDataEnd, vec![8, 42, 0x80]),
        proto_response(IncomingMessages::ContractDataEnd, vec![]),
        row(),
    ] {
        let registry = Registry::default();
        let read = registry.register(ID, spec()).unwrap();
        registry.get(ID).unwrap().push(message.into());
        assert!(read.terminal().is_none());
    }
    let registry = Registry::default();
    let read = registry
        .register(
            ID,
            RequestSpec {
                limits: RawLimits {
                    frame_bytes: 1,
                    ..spec().limits
                },
                ..spec()
            },
        )
        .unwrap();
    registry.get(ID).unwrap().push(end(ID).into());
    assert!(read.terminal().is_none());
}

#[test]
fn native_empty_samples_is_data_and_terminal_not_eof() {
    let registry = Registry::default();
    let mut read = registry
        .register(
            ID,
            RequestSpec {
                data: IncomingMessages::SymbolSamples,
                end: IncomingMessages::SymbolSamples,
                ..spec()
            },
        )
        .unwrap();
    registry
        .get(ID)
        .unwrap()
        .push(proto_response(IncomingMessages::SymbolSamples, symbol_samples().request_id(ID).encode_proto()).into());
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Response(_)))));
    assert!(matches!(read.take_next(), Some(None)));
    assert!(matches!(read.terminal(), Some(Terminal::End)));
}

#[test]
fn reset_and_stale_drop_preserve_replacement_and_shutdown_latches() {
    let registry = Registry::default();
    let mut old = registry.register(ID, spec()).unwrap();
    assert!(matches!(registry.register(ID, spec()), Err(Error::AlreadySubscribed)));
    registry.reset();
    assert!(matches!(old.take_next(), Some(Some(RoutedItem::Error(Error::ConnectionReset)))));
    let mut new = registry.register(ID, spec()).unwrap();
    drop(old);
    assert!(registry.get(ID).is_some());
    registry.close();
    assert!(matches!(new.take_next(), Some(Some(RoutedItem::Error(Error::Shutdown)))));
    assert!(matches!(registry.register(ID, spec()), Err(Error::Shutdown)));
    drop(new);
}

#[test]
fn rejects_invalid_registration_limits_and_ids() {
    let registry = Registry::default();
    assert!(matches!(registry.register(-1, spec()), Err(Error::InvalidArgument(_))));
    for limits in [
        RawLimits { frames: 0, ..spec().limits },
        RawLimits {
            frame_bytes: 0,
            ..spec().limits
        },
        RawLimits {
            total_bytes: 0,
            ..spec().limits
        },
    ] {
        assert!(matches!(
            registry.register(ID, RequestSpec { limits, ..spec() }),
            Err(Error::InvalidArgument(_))
        ));
    }
}

#[test]
fn notices_are_budgeted_and_interruption_preserves_prefix() {
    let registry = Registry::default();
    let mut read = registry.register(ID, spec()).unwrap();
    let inbox = registry.get(ID).unwrap();
    let notice = Notice {
        code: 2104,
        message: "synthetic advisory".into(),
        request_id: Some(ID),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    inbox.push(RoutedItem::Notice(notice.clone()));
    inbox.push(row().into());
    inbox.interrupt(Error::ConnectionReset);
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Notice(n))) if n == notice));
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Response(_)))));
    assert!(matches!(read.take_next(), Some(Some(RoutedItem::Error(Error::ConnectionReset)))));
}

#[test]
fn oversized_fatal_diagnostic_is_not_retained() {
    let registry = Registry::default();
    let read = registry.register(ID, spec()).unwrap();
    registry.get(ID).unwrap().push(
        Error::Notice(Notice {
            code: 200,
            advanced_order_reject_json: "x".repeat(spec().limits.frame_bytes + 1),
            request_id: Some(ID),
            message: String::new(),
            error_time: None,
        })
        .into(),
    );
    assert!(matches!(read.terminal(), Some(Terminal::Error(Error::ResponseLimitExceeded { .. }))));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn pending_reader_and_cleanup_are_woken_without_losing_queued_data() {
    let registry = Registry::default();
    let mut read = registry.register(ID, spec()).unwrap();
    let inbox = registry.get(ID).unwrap();
    let mut next = Box::pin(read.next_async());
    assert!(futures::poll!(next.as_mut()).is_pending());
    inbox.push(row().into());
    drop(next); // cancel-safe read: poll never removed a row
    assert!(matches!(read.next_async().await, Some(RoutedItem::Response(_))));
    read.discard_buffered();
    let mut terminal = Box::pin(read.terminal_async());
    assert!(futures::poll!(terminal.as_mut()).is_pending());
    inbox.push(end(ID).into());
    assert!(matches!(terminal.await, Terminal::End));
}

#[cfg(feature = "sync")]
#[test]
fn blocking_deadline_does_not_invent_end_or_consume_future_rows() {
    let registry = Registry::default();
    let mut read = registry.register(ID, spec()).unwrap();
    assert!(matches!(read.next_until(std::time::Instant::now()), Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::TimedOut));
    assert!(read.terminal().is_none());
    registry.get(ID).unwrap().push(row().into());
    assert!(matches!(read.next_until(std::time::Instant::now()), Ok(Some(RoutedItem::Response(_)))));
    assert!(matches!(read.terminal_until(std::time::Instant::now()), Err(Error::Io(_))));
    registry.get(ID).unwrap().push(end(ID).into());
    assert!(matches!(read.terminal_until(std::time::Instant::now()), Ok(Terminal::End)));
}
