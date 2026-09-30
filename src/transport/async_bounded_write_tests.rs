//! Bounded writes on the async bus: an uncertain (failed or abandoned) write
//! retires the session before any later writer can append to a partial frame.

use super::*;
use crate::messages::encode_raw_length;
use crate::testdata::builders::contracts::matching_symbols_request;
use crate::testdata::builders::RequestEncoder;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Default)]
enum WriteMode {
    #[default]
    Succeed,
    Fail,
    Pending,
}

/// Frame I/O seam: failed or suspended writes have already written a prefix.
/// No socket, background writer or live broker is involved.
#[derive(Clone, Debug, Default)]
struct SubmissionStream {
    inner: MemoryStream,
    mode: Arc<Mutex<WriteMode>>,
    attempts: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[async_trait]
impl AsyncIo for SubmissionStream {
    async fn read_message(&self) -> Result<Vec<u8>, Error> {
        self.inner.read_message().await
    }

    async fn write_all(&self, bytes: &[u8]) -> Result<(), Error> {
        self.attempts.lock().unwrap().push(bytes.to_vec());
        let mode = *self.mode.lock().unwrap();
        if matches!(mode, WriteMode::Succeed) {
            return self.inner.write_all(bytes).await;
        }
        self.inner.write_all(&bytes[..2]).await?;
        if matches!(mode, WriteMode::Fail) {
            Err(Error::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "synthetic write failure")))
        } else {
            std::future::pending().await
        }
    }
}

#[async_trait]
impl AsyncReconnect for SubmissionStream {
    async fn reconnect(&self) -> Result<(), Error> {
        self.inner.reconnect().await
    }

    async fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        self.inner.sleep(duration, shutdown).await;
    }
}

impl AsyncStream for SubmissionStream {}

#[tokio::test]
async fn bounded_partial_write_drop_retires_before_queued_legacy_writer_runs() {
    let (stream, bus) = make_bus(WriteMode::Pending);
    let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
    let mut bounded = Box::pin(bus.send_bounded(packet.clone()));
    assert!(futures::poll!(bounded.as_mut()).is_pending());
    assert_one_attempt(&stream, true);
    let mut queued = Box::pin(bus.send_message(packet));
    assert!(futures::poll!(queued.as_mut()).is_pending());
    drop(bounded);
    *stream.mode.lock().unwrap() = WriteMode::Succeed;
    assert!(matches!(queued.await, Err(Error::Shutdown)));
    assert!(bus.shutdown.is_requested());
    assert_one_attempt(&stream, true);
    assert!(matches!(bus.connection.handshake().await, Err(Error::Shutdown)));
    assert_one_attempt(&stream, true);
}

#[tokio::test]
async fn bounded_write_failure_retires_but_success_keeps_session_reusable() {
    for mode in [WriteMode::Fail, WriteMode::Succeed] {
        let (stream, bus) = make_bus(mode);
        let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
        let result = bus.send_bounded(packet.clone()).await;
        if matches!(mode, WriteMode::Fail) {
            assert!(matches!(result, Err(Error::Io(_))));
            *stream.mode.lock().unwrap() = WriteMode::Succeed;
            assert!(bus.shutdown.is_requested());
            // Retired: the bus's send gate now refuses every write (as `ConnectionReset`,
            // its answer for any session that is not live), so nothing reaches the socket.
            assert!(matches!(bus.send_message(packet).await, Err(Error::ConnectionReset)));
            assert_one_attempt(&stream, true);
        } else {
            result.unwrap();
            assert!(!bus.shutdown.is_requested());
            assert_one_attempt(&stream, false);
        }
    }
}

#[tokio::test]
async fn bounded_public_start_drop_and_cancel_drop_close_owned_registration() {
    use crate::contracts::{Contract, QueryDisposition, QueryLimits};
    for cancel in [false, true] {
        let (stream, bus) = make_bus(if cancel { WriteMode::Succeed } else { WriteMode::Pending });
        let client = crate::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
        let mut query = client
            .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
            .unwrap();
        let id = query.request_id();
        if cancel {
            query.start().await.unwrap();
            *stream.mode.lock().unwrap() = WriteMode::Pending;
            let mut write = Box::pin(query.request_cancel());
            assert!(futures::poll!(write.as_mut()).is_pending());
            drop(write);
        } else {
            let mut write = Box::pin(query.start());
            assert!(futures::poll!(write.as_mut()).is_pending());
            drop(write);
        }
        assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
        assert!(!bus.is_connected());
        assert!(bus.bounded_requests.get(id).is_none());
        assert_eq!(stream.attempts.lock().unwrap().len(), if cancel { 2 } else { 1 });
        *stream.mode.lock().unwrap() = WriteMode::Succeed;
        // Retired: the bus's send gate now refuses every write (as `ConnectionReset`,
        // its answer for any session that is not live), so nothing reaches the socket.
        assert!(matches!(bus.send_message(vec![]).await, Err(Error::ConnectionReset)));
    }
}

#[tokio::test]
async fn bounded_public_failed_write_retires_without_retry() {
    let (_, bus) = make_bus(WriteMode::Fail);
    let client = crate::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .prepare_matching_symbols("SYNTH", crate::contracts::QueryLimits::default())
        .unwrap();
    let id = query.request_id();
    assert!(matches!(query.start().await, Err(Error::Io(_))));
    assert_eq!(query.disposition(), crate::contracts::QueryDisposition::RetireRequired);
    assert!(bus.bounded_requests.get(id).is_none());
    assert!(matches!(query.next().await, Err(Error::Shutdown)));
    assert!(matches!(query.request_cancel().await, Err(Error::Shutdown)));
    assert!(matches!(query.drain_until(tokio::time::Instant::now()).await, Err(Error::Shutdown)));
}

#[tokio::test]
async fn bounded_failed_cancel_keeps_buffered_prefix_readable_then_fuses() {
    use crate::common::test_utils::helpers::binary_proto;
    use crate::contracts::{Contract, QueryLimits};
    use crate::subscriptions::SubscriptionItem;
    use crate::testdata::builders::contracts::contract_data;
    use crate::testdata::builders::ResponseProtoEncoder;
    let (stream, bus) = make_bus(WriteMode::Succeed);
    let client = crate::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
        .unwrap();
    query.start().await.unwrap();
    stream.inner.push_inbound(binary_proto(
        IncomingMessages::ContractData as i32,
        &contract_data().request_id(query.request_id()).contract_id(123).to_proto(),
    ));
    bus.read_and_route_message().await.unwrap();
    *stream.mode.lock().unwrap() = WriteMode::Fail;
    assert!(matches!(query.request_cancel().await, Err(Error::Io(_))));
    assert!(!bus.is_connected());
    assert!(matches!(query.next().await.unwrap(), Some(SubscriptionItem::Data(row)) if row.contract.contract_id == 123));
    assert!(matches!(query.next().await, Err(Error::Shutdown)));
    assert!(query.next().await.unwrap().is_none());
}

fn make_bus(mode: WriteMode) -> (SubmissionStream, Arc<AsyncTcpMessageBus<SubmissionStream>>) {
    let stream = SubmissionStream::default();
    *stream.mode.lock().unwrap() = mode;
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    (stream, Arc::new(AsyncTcpMessageBus::new(connection).unwrap()))
}

fn assert_one_attempt(stream: &SubmissionStream, partial: bool) {
    let expected = encode_raw_length(&matching_symbols_request().request_id(42).pattern("AAP").encode_request());
    assert_eq!(stream.attempts.lock().unwrap().as_slice(), std::slice::from_ref(&expected));
    assert_eq!(stream.inner.captured(), if partial { expected[..2].to_vec() } else { expected });
}

/// A failed bounded write retires the session at once, but only latches the
/// shutdown flag: the registry closes when the dispatcher finishes the
/// shutdown. A query prepared in between must still be refused.
#[tokio::test]
async fn bounded_prepare_after_a_retiring_write_is_refused() {
    let (stream, bus) = make_bus(WriteMode::Fail);
    let client = crate::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
    assert!(matches!(bus.send_bounded(packet).await, Err(Error::Io(_))));
    assert!(bus.shutdown.is_requested());

    let prepared = client.prepare_matching_symbols("SYNTH", crate::contracts::QueryLimits::default());

    assert!(matches!(prepared, Err(Error::Shutdown)));
    assert_eq!(stream.attempts.lock().unwrap().len(), 1, "only the retiring write");
}

/// While the session is reconnecting the send gate refuses a start before
/// writing: the query stays not submitted, the session is not retired, and
/// the same query starts once the session is live again.
#[tokio::test]
async fn bounded_start_refused_while_reconnecting_does_not_retire() {
    use crate::contracts::{QueryDisposition, QueryLimits};
    let (stream, bus) = make_bus(WriteMode::Succeed);
    let client = crate::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    bus.connection_state.set_disconnected();
    let mut query = client.prepare_matching_symbols("SYNTH", QueryLimits::default()).unwrap();

    assert!(matches!(query.start().await, Err(Error::ConnectionReset)));
    assert_eq!(query.disposition(), QueryDisposition::NotSubmitted);
    assert!(!bus.shutdown.is_requested(), "a refusal before writing retires nothing");
    assert!(stream.attempts.lock().unwrap().is_empty());

    bus.connection_state.set_connected();
    query.start().await.unwrap();
    assert_eq!(query.disposition(), QueryDisposition::Pending);
    assert_eq!(stream.attempts.lock().unwrap().len(), 1);
}
