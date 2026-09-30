//! Bounded writes on the blocking bus: a failed write retires the session
//! before any later write or reconnect handshake.

use super::*;
use crate::messages::encode_raw_length;
use crate::testdata::builders::contracts::matching_symbols_request;
use crate::testdata::builders::RequestEncoder;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Default)]
struct SubmissionStream {
    inner: MemoryStream,
    fail: Arc<AtomicBool>,
    attempts: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl Io for SubmissionStream {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        self.inner.read_message()
    }

    fn write_all(&self, bytes: &[u8]) -> Result<(), Error> {
        self.attempts.lock().unwrap().push(bytes.to_vec());
        if self.fail.load(Ordering::SeqCst) {
            self.inner.write_all(&bytes[..2])?;
            return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "synthetic write failure")));
        }
        self.inner.write_all(bytes)
    }
}

impl Reconnect for SubmissionStream {
    fn reconnect(&self) -> Result<(), Error> {
        self.inner.reconnect()
    }

    fn sleep(&self, duration: std::time::Duration, shutdown: &ShutdownSignal) {
        self.inner.sleep(duration, shutdown);
    }

    fn shutdown_read(&self) -> Result<(), Error> {
        self.inner.shutdown_read()
    }
}

impl Stream for SubmissionStream {}

#[test]
fn bounded_write_failure_retires_before_later_writes_or_handshake() {
    let (stream, bus) = make_bus(true);
    let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
    assert!(matches!(bus.send_bounded(&packet), Err(Error::Io(_))));
    stream.fail.store(false, Ordering::SeqCst);
    // Retired: the bus's send gate now refuses every write (as `ConnectionReset`,
    // its answer for any session that is not live), so nothing reaches the socket.
    assert!(matches!(bus.send_message(&packet), Err(Error::ConnectionReset)));
    assert!(matches!(bus.connection.handshake(), Err(Error::Shutdown)));
    assert_eq!(stream.attempts.lock().unwrap().len(), 1);
    assert_eq!(stream.inner.captured(), encode_raw_length(&packet)[..2]);
}

#[test]
fn bounded_success_does_not_retire_client() {
    let (stream, bus) = make_bus(false);
    let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
    bus.send_bounded(&packet).unwrap();
    assert!(!bus.shutdown.is_requested());
    assert_eq!(stream.inner.captured(), encode_raw_length(&packet));
}

#[test]
fn bounded_public_start_or_cancel_failure_retires_without_retry() {
    use crate::contracts::{Contract, QueryDisposition, QueryLimits};
    for cancel in [false, true] {
        let (stream, bus) = make_bus(!cancel);
        let bus = Arc::new(bus);
        let client = crate::client::blocking::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
        let mut query = client
            .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
            .unwrap();
        let id = query.request_id();
        if cancel {
            query.start().unwrap();
            stream.fail.store(true, Ordering::SeqCst);
            assert!(matches!(query.request_cancel(), Err(Error::Io(_))));
        } else {
            assert!(matches!(query.start(), Err(Error::Io(_))));
        }
        assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
        assert!(!bus.is_connected());
        assert!(bus.bounded_requests.get(id).is_none());
        assert_eq!(stream.attempts.lock().unwrap().len(), if cancel { 2 } else { 1 });
        assert!(matches!(query.next_until(std::time::Instant::now()), Err(Error::Shutdown)));
        assert!(matches!(query.request_cancel(), Err(Error::Shutdown)));
        assert!(matches!(query.drain_until(std::time::Instant::now()), Err(Error::Shutdown)));
    }
}

#[test]
fn bounded_failed_cancel_keeps_buffered_prefix_readable_then_fuses() {
    use crate::common::test_utils::helpers::binary_proto;
    use crate::contracts::{Contract, QueryLimits};
    use crate::subscriptions::SubscriptionItem;
    use crate::testdata::builders::contracts::contract_data;
    use crate::testdata::builders::ResponseProtoEncoder;
    let (stream, bus) = make_bus(false);
    let bus = Arc::new(bus);
    let client = crate::client::blocking::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
        .unwrap();
    query.start().unwrap();
    stream.inner.push_inbound(binary_proto(
        IncomingMessages::ContractData as i32,
        &contract_data().request_id(query.request_id()).contract_id(123).to_proto(),
    ));
    bus.dispatch().unwrap();
    stream.fail.store(true, Ordering::SeqCst);
    assert!(matches!(query.request_cancel(), Err(Error::Io(_))));
    assert!(!bus.is_connected());
    assert!(matches!(query.next_until(std::time::Instant::now()).unwrap(), Some(SubscriptionItem::Data(row)) if row.contract.contract_id == 123));
    assert!(matches!(query.next_until(std::time::Instant::now()), Err(Error::Shutdown)));
    assert!(query.next_until(std::time::Instant::now()).unwrap().is_none());
}

fn make_bus(fail: bool) -> (SubmissionStream, TcpMessageBus<SubmissionStream>) {
    let stream = SubmissionStream::default();
    stream.fail.store(fail, Ordering::SeqCst);
    let connection = Connection::stubbed(stream.clone(), 28);
    (stream, TcpMessageBus::new(connection).unwrap())
}

/// A failed bounded write retires the session at once, but only latches the
/// shutdown flag; the registry closes when the bus finishes the shutdown. A
/// query prepared in between must still be refused.
#[test]
fn bounded_prepare_after_a_retiring_write_is_refused() {
    let (stream, bus) = make_bus(true);
    let bus = Arc::new(bus);
    let client = crate::client::blocking::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    let packet = matching_symbols_request().request_id(42).pattern("AAP").encode_request();
    assert!(matches!(bus.send_bounded(&packet), Err(Error::Io(_))));
    assert!(bus.shutdown.is_requested());

    let prepared = client.prepare_matching_symbols("SYNTH", crate::contracts::QueryLimits::default());

    assert!(matches!(prepared, Err(Error::Shutdown)));
    assert_eq!(stream.attempts.lock().unwrap().len(), 1, "only the retiring write");
}

/// While the session is reconnecting the send gate refuses a start before
/// writing: the query stays not submitted, the session is not retired, and
/// the same query starts once the session is live again.
#[test]
fn bounded_start_refused_while_reconnecting_does_not_retire() {
    use crate::contracts::{QueryDisposition, QueryLimits};
    let (stream, bus) = make_bus(false);
    let bus = Arc::new(bus);
    let client = crate::client::blocking::Client::stubbed(bus.clone(), crate::server_versions::CANCEL_CONTRACT_DATA);
    bus.connection_state.set_disconnected();
    let mut query = client.prepare_matching_symbols("SYNTH", QueryLimits::default()).unwrap();

    assert!(matches!(query.start(), Err(Error::ConnectionReset)));
    assert_eq!(query.disposition(), QueryDisposition::NotSubmitted);
    assert!(!bus.shutdown.is_requested(), "a refusal before writing retires nothing");
    assert!(stream.attempts.lock().unwrap().is_empty());

    bus.connection_state.set_connected();
    query.start().unwrap();
    assert_eq!(query.disposition(), QueryDisposition::Pending);
    assert_eq!(stream.attempts.lock().unwrap().len(), 1);
}
