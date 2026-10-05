use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex, RwLock,
    },
};

#[cfg(feature = "sync")]
use crossbeam::channel;

use crate::accounts::types::AccountId;
use crate::client::ids::{OrderId, RequestId};
use crate::messages::{OutgoingMessages, ResponseMessage};
use crate::transport::common::Lease;
use crate::transport::routing::{classify_error, determine_routing, ErrorDisposition, RoutingDecision};
use crate::transport::{RoutedItem, SharedTicket};
use crate::Error;

#[cfg(feature = "sync")]
use crate::transport::{InternalSubscription, MessageBus, Signal, SubscriptionBuilder};

#[cfg(feature = "async")]
use {
    crate::transport::{
        r#async::{AsyncInternalSubscription, CleanupSignal, RouteKey},
        AsyncMessageBus,
    },
    async_trait::async_trait,
    tokio::sync::broadcast,
};

#[cfg(feature = "async")]
const TEST_BROADCAST_CAPACITY: usize = 1024;

pub(crate) struct MessageBusStub {
    pub request_messages: RwLock<Vec<Vec<u8>>>,
    pub response_messages: Vec<String>,
    /// Pre-built responses, served in order. When non-empty, supersedes
    /// `response_messages`.
    pub ordered_responses: Vec<ResponseMessage>,
    /// The `limit` of each `send_request_bounded` call, in order. The stub
    /// doesn't enforce it; overflow is tested on the real buses.
    pub buffer_limits: RwLock<Vec<usize>>,
    /// The request id of each `send_executions_request` call, in order.
    #[cfg(feature = "async")]
    pub executions_requests: RwLock<Vec<RequestId>>,
    /// Requests still to be answered with [`Error::ConnectionReset`] before the
    /// configured responses are served. See [`MessageBusStub::with_connection_resets`].
    connection_resets: AtomicUsize,
    /// The runtime the stub was built on, if any; sync-only tests build it
    /// with none. See [`AsyncMessageBus::runtime_handle`].
    #[cfg(feature = "async")]
    runtime: Option<tokio::runtime::Handle>,
    /// The cleanup channel stubbed subscriptions signal on drop or cancel.
    /// Nothing reads it; the receiver is kept so sends succeed rather than
    /// logging a warning per subscription.
    #[cfg(feature = "sync")]
    signals: (channel::Sender<Signal>, channel::Receiver<Signal>),
    // pub next_request_id: i32,
    // pub server_version: i32,
    // pub order_id: i32,
}

// Separate tracking for order update subscriptions to maintain backward compatibility
static ORDER_UPDATE_SUBSCRIPTION_TRACKER: LazyLock<Mutex<HashSet<usize>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

impl Default for MessageBusStub {
    fn default() -> Self {
        Self {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
            ordered_responses: vec![],
            buffer_limits: RwLock::new(vec![]),
            #[cfg(feature = "async")]
            executions_requests: RwLock::new(vec![]),
            connection_resets: AtomicUsize::new(0),
            #[cfg(feature = "async")]
            runtime: tokio::runtime::Handle::try_current().ok(),
            #[cfg(feature = "sync")]
            signals: channel::unbounded(),
        }
    }
}

impl Drop for MessageBusStub {
    fn drop(&mut self) {
        // Clean up the subscription tracker to prevent test isolation issues
        let stub_id = self as *const _ as usize;
        ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().remove(&stub_id);
    }
}

impl MessageBusStub {
    pub fn with_responses(response_messages: Vec<String>) -> Self {
        let mut stub = Self::default();
        stub.response_messages = response_messages;
        stub
    }

    /// Construct a stub that plays back an ordered sequence of pre-built
    /// `ResponseMessage` values, typically from the proto builders in
    /// `testdata::builders`.
    pub fn with_ordered_responses(ordered_responses: Vec<ResponseMessage>) -> Self {
        let mut stub = Self::default();
        stub.ordered_responses = ordered_responses;
        stub
    }

    /// Answer the first `count` requests with [`Error::ConnectionReset`] before
    /// serving the configured responses.
    ///
    /// The real transport synthesizes `ConnectionReset` in its reconnect path
    /// (`fail_all`), which the stub bypasses — so before this existed, no test
    /// could reach the retry wiring every one-shot request goes through (#741),
    /// only the combinator in `src/common/retry.rs`. The assertion is a resend
    /// count: `request_messages().len()` is the number of attempts, because each
    /// retry re-encodes and re-sends.
    ///
    /// Same move as #735 made for error frames — a fixture that lies about what
    /// the wire produces makes the tests that depend on it worthless.
    pub fn with_connection_resets(self, count: usize) -> Self {
        self.connection_resets.store(count, Ordering::SeqCst);
        self
    }

    pub fn request_messages(&self) -> Vec<Vec<u8>> {
        self.request_messages.read().unwrap().clone()
    }

    /// Materialise configured responses as `ResponseMessage` instances.
    /// Prefers `ordered_responses` over the legacy text-only
    /// `response_messages` field; only one is non-empty per test.
    pub(crate) fn response_messages_decoded(&self) -> Vec<ResponseMessage> {
        if !self.ordered_responses.is_empty() {
            return self.ordered_responses.clone();
        }
        self.response_messages
            .iter()
            .map(|m| ResponseMessage::from(&m.replace('|', "\0")))
            .collect()
    }

    /// Configured responses as the dispatcher would deliver them.
    ///
    /// The stub has no dispatcher, so it used to hand every fixture over as
    /// `RoutedItem::Response` — including `Error` frames, which the real
    /// transport never does. That gap is why decoders grew unreachable
    /// `IncomingMessages::Error` arms and why tests asserting them passed:
    /// only a stub could produce the input. Classifying here keeps the
    /// fixture-side contract the same as the wire-side one.
    ///
    /// This is the same blind spot `debug_assert_request_id_routable` covers
    /// from the routing side — stub tests inject below `determine_routing`.
    pub(crate) fn routed_items(&self) -> Vec<RoutedItem> {
        self.response_messages_decoded().into_iter().map(classify_like_dispatcher).collect()
    }

    /// What one request's subscription receives.
    ///
    /// Identical to [`Self::routed_items`] unless the stub was built with
    /// [`Self::with_connection_resets`], in which case the leading requests get
    /// a reset instead — the transport delivers one to every in-flight
    /// subscription when the socket drops, and delivers no responses at all.
    fn routed_items_for_request(&self) -> Vec<RoutedItem> {
        let remaining = self.connection_resets.load(Ordering::SeqCst);
        if remaining > 0 {
            self.connection_resets.store(remaining - 1, Ordering::SeqCst);
            return vec![RoutedItem::Error(Error::ConnectionReset)];
        }
        self.routed_items()
    }

    /// Record the outbound request and hand back a subscription pre-loaded with
    /// the configured responses. Every async `send_*` differs only in the id or
    /// message-type argument it ignores. The subscription carries no cleanup
    /// signal, so a shared request built through the stub is not tagged with
    /// its request type and its drop does not take the shared-cancel path.
    #[cfg(feature = "async")]
    fn seeded_subscription(&self, message: Vec<u8>) -> AsyncInternalSubscription {
        self.request_messages.write().unwrap().push(message);

        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);
        for item in self.routed_items_for_request() {
            sender.send(item).unwrap();
        }

        AsyncInternalSubscription::new(receiver)
    }
}

/// Apply the dispatcher's classification to a fixture. The stub has no channel
/// map, so the routing *target* is irrelevant here — only the classification is,
/// and it covers both types the dispatcher intercepts before routing
/// (`DISPATCHER_INTERCEPTED` in `src/subscriptions/response_message_ids_tests.rs`).
fn classify_like_dispatcher(message: ResponseMessage) -> RoutedItem {
    match determine_routing(&message) {
        RoutingDecision::Error(payload) => match classify_error(payload) {
            // A request-scoped error or warning goes to its subscription.
            ErrorDisposition::Route(_, item) => item,
            // Request-less: a warning is informational, a hard error fails the
            // in-flight one-shot. The stub has exactly one channel, so both
            // land here rather than on a notice broadcaster.
            ErrorDisposition::NoticeOnly(notice) => RoutedItem::Notice(notice),
            ErrorDisposition::NoticeAndFailOneShots(_, error) => RoutedItem::Error(error),
        },
        // Ends the dispatcher loop on a real transport, so it never reaches a
        // decoder there either.
        RoutingDecision::Shutdown => RoutedItem::Error(Error::Shutdown),
        _ => message.into(),
    }
}

#[cfg(feature = "sync")]
impl MessageBus for MessageBusStub {
    fn send_request(&self, request_id: RequestId, message: &[u8]) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, MockRoute::Request(request_id), message))
    }

    fn send_request_bounded(
        &self,
        request_id: RequestId,
        message: &[u8],
        bound: crate::transport::BufferBound,
    ) -> Result<InternalSubscription, Error> {
        self.buffer_limits.write().unwrap().push(bound.limit);
        Ok(mock_request(self, MockRoute::Request(request_id), message))
    }

    fn send_order_request(&self, order_id: OrderId, message: &[u8]) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, MockRoute::Order(order_id), message))
    }

    fn send_message(&self, message: &[u8]) -> Result<(), Error> {
        self.request_messages.write().unwrap().push(message.to_vec());
        Ok(())
    }

    fn create_order_update_subscription(&self) -> Result<InternalSubscription, Error> {
        // Use pointer address as unique identifier for this stub instance
        let stub_id = self as *const _ as usize;

        let mut tracker = ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap();
        if !tracker.insert(stub_id) {
            return Err(Error::AlreadySubscribed);
        }
        drop(tracker); // Release lock early

        let (sender, receiver) = channel::unbounded();

        // Send any pre-configured response messages
        for item in self.routed_items() {
            sender.send(item).unwrap();
        }

        let subscription = SubscriptionBuilder::new()
            .receiver(receiver)
            .signaler(self.signals.0.clone())
            .lease(Lease::new())
            .build();

        Ok(subscription)
    }

    fn send_shared_request(&self, message_type: OutgoingMessages, message: &[u8]) -> Result<InternalSubscription, Error> {
        Ok(mock_request(self, MockRoute::Shared(message_type), message))
    }

    fn send_account_updates_request(&self, _account: &AccountId, message: &[u8]) -> Result<InternalSubscription, Error> {
        MessageBus::send_shared_request(self, OutgoingMessages::RequestAccountData, message)
    }

    fn cancel_shared_subscription(&self, _ticket: SharedTicket, packet: Option<&[u8]>) -> Result<(), Error> {
        if let Some(packet) = packet {
            self.request_messages.write().unwrap().push(packet.to_vec());
        }
        Ok(())
    }

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::sync_impl::NoticeStream {
        // No global notices delivered through the stub; hand back an empty,
        // already-closed channel so callers see end-of-stream cleanly.
        let (_sender, receiver) = channel::unbounded();
        crate::subscriptions::notice_stream::sync_impl::NoticeStream::new(receiver)
    }

    fn ensure_shutdown(&self) {}

    fn wait_connected(&self) -> Result<(), Error> {
        Ok(()) // Stub is always connected
    }

    fn is_connected(&self) -> bool {
        true // Stub always returns connected
    }

    // fn process_messages(&mut self, _server_version: i32) -> Result<(), Error> {
    //     Ok(())
    // }
}

#[cfg(feature = "sync")]
enum MockRoute {
    Request(RequestId),
    Order(OrderId),
    Shared(OutgoingMessages),
}

#[cfg(feature = "sync")]
fn mock_request(stub: &MessageBusStub, route: MockRoute, message: &[u8]) -> InternalSubscription {
    stub.request_messages.write().unwrap().push(message.to_vec());

    let (sender, receiver) = channel::unbounded();

    for item in stub.routed_items_for_request() {
        sender.send(item).unwrap();
    }

    let subscription = SubscriptionBuilder::new()
        .signaler(stub.signals.0.clone())
        .lease(Lease::new())
        .receiver(receiver);
    match route {
        MockRoute::Request(request_id) => subscription.request_id(request_id),
        MockRoute::Order(order_id) => subscription.order_id(order_id),
        MockRoute::Shared(message_type) => subscription.shared(SharedTicket { message_type, generation: 0 }),
    }
    .build()
}

#[cfg(feature = "async")]
#[async_trait]
impl AsyncMessageBus for MessageBusStub {
    async fn send_request(&self, _request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        Ok(self.seeded_subscription(message))
    }

    async fn send_request_bounded(
        &self,
        _request_id: RequestId,
        message: Vec<u8>,
        bound: crate::transport::BufferBound,
    ) -> Result<AsyncInternalSubscription, Error> {
        self.buffer_limits.write().unwrap().push(bound.limit);
        Ok(self.seeded_subscription(message))
    }

    async fn send_executions_request(&self, request_id: RequestId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        self.executions_requests.write().unwrap().push(request_id);
        Ok(self.seeded_subscription(message))
    }

    async fn send_order_request(&self, _order_id: OrderId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        Ok(self.seeded_subscription(message))
    }

    async fn send_shared_request(&self, _message_type: OutgoingMessages, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        Ok(self.seeded_subscription(message))
    }

    async fn send_account_updates_request(&self, _account: &AccountId, message: Vec<u8>) -> Result<AsyncInternalSubscription, Error> {
        AsyncMessageBus::send_shared_request(self, OutgoingMessages::RequestAccountData, message).await
    }

    async fn send_message(&self, message: Vec<u8>) -> Result<(), Error> {
        self.request_messages.write().unwrap().push(message);
        Ok(())
    }

    async fn cancel_shared_subscription(&self, _ticket: SharedTicket, message: Option<Vec<u8>>) -> Result<(), Error> {
        if let Some(message) = message {
            self.request_messages.write().unwrap().push(message);
        }
        Ok(())
    }

    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        let stub_id = self as *const _ as usize;
        let mut tracker = ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap();
        if !tracker.insert(stub_id) {
            return Err(Error::AlreadySubscribed);
        }
        drop(tracker);

        let (sender, receiver) = broadcast::channel(TEST_BROADCAST_CAPACITY);

        // Send pre-configured response messages
        for item in self.routed_items() {
            sender.send(item).unwrap();
        }

        let (cleanup_sender, mut cleanup_receiver) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(signal) = cleanup_receiver.recv().await {
                if matches!(signal, CleanupSignal::OrderUpdateStream(_)) {
                    ORDER_UPDATE_SUBSCRIPTION_TRACKER.lock().unwrap().remove(&stub_id);
                    break;
                }
            }
        });

        Ok(AsyncInternalSubscription::with_cleanup(receiver, cleanup_sender).leased(Lease::new(), RouteKey::OrderUpdateStream))
    }

    fn notice_subscribe(&self) -> crate::subscriptions::notice_stream::async_impl::NoticeStream {
        // No global notices delivered through the stub; the broadcast channel
        // closes immediately so callers see end-of-stream cleanly.
        let (_sender, receiver) = broadcast::channel(1);
        crate::subscriptions::notice_stream::async_impl::NoticeStream::new(receiver)
    }

    async fn ensure_shutdown(&self) {
        // No-op for test stub
    }

    fn request_shutdown_sync(&self) {
        // No-op for test stub
    }

    async fn wait_connected(&self) -> Result<(), Error> {
        Ok(()) // Stub is always connected
    }

    fn is_connected(&self) -> bool {
        true // Stub always returns connected
    }

    fn runtime_handle(&self) -> &tokio::runtime::Handle {
        self.runtime.as_ref().expect("MessageBusStub built outside a Tokio runtime")
    }
}

#[cfg(test)]
#[path = "stubs_tests.rs"]
mod tests;
