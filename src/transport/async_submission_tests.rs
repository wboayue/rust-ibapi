//! Registration ownership across an async submission's write: what a caller
//! that abandons `send_request` / `send_request_bounded` /
//! `send_order_request` mid-write leaves behind.

use super::tests::drain_cleanup_signals;
use super::*;
use crate::messages::encode_raw_length;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Default)]
enum WriteMode {
    #[default]
    Succeed,
    /// Writes a prefix of the frame, then never completes.
    Pending,
}

/// A stream whose writes can be held pending after a partial frame, the way a
/// socket write blocks on a full send buffer. Reads come from the inner
/// `MemoryStream`.
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
        match mode {
            WriteMode::Succeed => self.inner.write_all(bytes).await,
            WriteMode::Pending => {
                self.inner.write_all(&bytes[..2]).await?;
                std::future::pending().await
            }
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

const ID: i32 = 42;

fn packet() -> Vec<u8> {
    b"submission".to_vec()
}

#[derive(Clone, Copy, Debug)]
enum Registration {
    Request,
    BoundedRequest,
    Order,
}

const KINDS: [Registration; 3] = [Registration::Request, Registration::BoundedRequest, Registration::Order];

impl Registration {
    /// The sender registered under `ID`, if any.
    async fn registered(self, bus: &AsyncTcpMessageBus<SubmissionStream>) -> Option<BroadcastSender> {
        match self {
            Self::Request | Self::BoundedRequest => bus.request_channels.read().await.get(&ID).map(|route| route.sender.clone()),
            Self::Order => bus.order_channels.read().await.get(&ID).cloned(),
        }
    }

    async fn submit(self, bus: &AsyncTcpMessageBus<SubmissionStream>) -> Result<AsyncInternalSubscription, Error> {
        match self {
            Self::Request => bus.send_request(ID, packet()).await,
            Self::BoundedRequest => {
                let bound = BufferBound {
                    limit: 8,
                    end: crate::messages::IncomingMessages::ContractDataEnd,
                };
                bus.send_request_bounded(ID, packet(), bound).await
            }
            Self::Order => bus.send_order_request(ID, packet()).await,
        }
    }
}

fn make_bus(mode: WriteMode) -> (SubmissionStream, Arc<AsyncTcpMessageBus<SubmissionStream>>) {
    let stream = SubmissionStream::default();
    *stream.mode.lock().unwrap() = mode;
    let connection = AsyncConnection::stubbed(stream.clone(), 28);
    (stream, Arc::new(AsyncTcpMessageBus::new(connection).unwrap()))
}

/// A caller that drops the submission while its write is pending (a timeout,
/// a losing `select!` branch) leaves no registration behind.
#[tokio::test]
async fn test_dropping_a_pending_write_releases_its_registration() {
    for kind in KINDS {
        let (stream, bus) = make_bus(WriteMode::Pending);
        let mut submitting = Box::pin(kind.submit(&bus));
        assert!(futures::poll!(submitting.as_mut()).is_pending());
        assert!(kind.registered(&bus).await.is_some(), "{kind:?}: registered before the write");
        assert_eq!(
            stream.inner.captured(),
            encode_raw_length(&packet())[..2].to_vec(),
            "{kind:?}: write in progress"
        );

        drop(submitting);
        *stream.mode.lock().unwrap() = WriteMode::Succeed;
        drain_cleanup_signals(&bus).await;

        assert!(
            kind.registered(&bus).await.is_none(),
            "{kind:?}: an abandoned write leaked its registration"
        );
    }
}

/// The abandoned submission's cleanup signal can run after a new submission
/// has registered under the same id; it must leave that registration alone.
#[tokio::test]
async fn test_abandoned_write_cleanup_preserves_a_newer_registration() {
    for kind in KINDS {
        let (stream, bus) = make_bus(WriteMode::Pending);
        // Hold the FIFO cleanup task on an unrelated map, so the abandoned
        // submission's signal cannot run until the replacement is live.
        let cleanup_gate = bus.order_update_stream.write().await;
        bus.cleanup_sender.send(CleanupSignal::OrderUpdateStream).unwrap();

        let mut submitting = Box::pin(kind.submit(&bus));
        assert!(futures::poll!(submitting.as_mut()).is_pending());
        drop(submitting);
        *stream.mode.lock().unwrap() = WriteMode::Succeed;
        let mut replacement = kind.submit(&bus).await.unwrap();
        drop(cleanup_gate);
        drain_cleanup_signals(&bus).await;

        let sender = kind
            .registered(&bus)
            .await
            .unwrap_or_else(|| panic!("{kind:?}: the stale cleanup removed the replacement"));
        sender.send(RoutedItem::Error(Error::Cancelled)).unwrap();
        assert!(matches!(replacement.try_next_routed(), Some(RoutedItem::Error(Error::Cancelled))));
    }
}

/// A completed write hands the registration to the returned subscription: it
/// stays while the subscription lives and goes when it is dropped.
#[tokio::test]
async fn test_a_completed_write_hands_cleanup_to_the_subscription() {
    for kind in KINDS {
        let (stream, bus) = make_bus(WriteMode::Succeed);
        let subscription = kind.submit(&bus).await.unwrap();
        assert_eq!(stream.inner.captured(), encode_raw_length(&packet()), "{kind:?}: whole frame written");

        drain_cleanup_signals(&bus).await;
        assert!(kind.registered(&bus).await.is_some(), "{kind:?}: live registration removed");

        drop(subscription);
        drain_cleanup_signals(&bus).await;
        assert!(kind.registered(&bus).await.is_none(), "{kind:?}: dropped subscription leaked");
    }
}
