//! Asynchronous transport implementation

use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task;
use tokio::time::{sleep, Duration};

/// Cleanup signal for removing channels when subscriptions are dropped
#[derive(Debug, Clone)]
pub enum CleanupSignal {
    Request(i32),
    Order(i32),
    Shared(OutgoingMessages),
}

use crate::connection::r#async::AsyncConnection;
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::Error;

use super::routing::{determine_routing, is_warning_error, map_incoming_to_outgoing, RoutingDecision, UNSPECIFIED_REQUEST_ID};

/// Asynchronous message bus trait
#[async_trait]
pub trait AsyncMessageBus: Send + Sync {
    /// Atomic subscribe + send for requests with IDs
    async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;

    /// Atomic subscribe + send for orders
    async fn send_order_request(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;

    /// Atomic subscribe + send for shared channels
    async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error>;

    /// Send without expecting response
    async fn send_message(&self, message: RequestMessage) -> Result<(), Error>;

    /// Cancel operations
    #[allow(dead_code)]
    async fn cancel_subscription(&self, request_id: i32, message: RequestMessage) -> Result<(), Error>;
    #[allow(dead_code)]
    async fn cancel_order_subscription(&self, order_id: i32, message: RequestMessage) -> Result<(), Error>;

    /// Order update stream
    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error>;

    #[cfg(test)]
    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
    }
}

/// Internal subscription for async implementation
pub struct AsyncInternalSubscription {
    pub(crate) receiver: mpsc::UnboundedReceiver<ResponseMessage>,
    cleanup_sender: Option<mpsc::UnboundedSender<CleanupSignal>>,
    cleanup_signal: Option<CleanupSignal>,
    cleanup_sent: bool,
}

impl AsyncInternalSubscription {
    pub fn new(receiver: mpsc::UnboundedReceiver<ResponseMessage>) -> Self {
        Self {
            receiver,
            cleanup_sender: None,
            cleanup_signal: None,
            cleanup_sent: false,
        }
    }

    pub fn with_cleanup(
        receiver: mpsc::UnboundedReceiver<ResponseMessage>,
        cleanup_sender: mpsc::UnboundedSender<CleanupSignal>,
        cleanup_signal: CleanupSignal,
    ) -> Self {
        Self {
            receiver,
            cleanup_sender: Some(cleanup_sender),
            cleanup_signal: Some(cleanup_signal),
            cleanup_sent: false,
        }
    }

    pub async fn next(&mut self) -> Option<ResponseMessage> {
        self.receiver.recv().await
    }

    /// Extract the receiver for use in subscriptions (disables cleanup)
    pub fn take_receiver(mut self) -> mpsc::UnboundedReceiver<ResponseMessage> {
        // Disable cleanup by clearing the cleanup info - the subscription will now own the receiver
        self.cleanup_sender = None;
        self.cleanup_signal = None;
        self.cleanup_sent = true; // Mark as sent to prevent Drop from sending

        // Create a dummy receiver to replace the original one
        let (_, dummy_receiver) = mpsc::unbounded_channel();
        mem::replace(&mut self.receiver, dummy_receiver)
    }

    /// Manually send cleanup signal
    fn send_cleanup_signal(&mut self) {
        if !self.cleanup_sent {
            if let (Some(sender), Some(signal)) = (&self.cleanup_sender, &self.cleanup_signal) {
                let _ = sender.send(signal.clone());
                self.cleanup_sent = true;
            }
        }
    }
}

/// Send cleanup signal when subscription is dropped
impl Drop for AsyncInternalSubscription {
    fn drop(&mut self) {
        self.send_cleanup_signal();
    }
}

type ChannelSender = mpsc::UnboundedSender<ResponseMessage>;
type SharedChannelSender = broadcast::Sender<ResponseMessage>;

/// Asynchronous TCP message bus implementation
pub struct AsyncTcpMessageBus {
    connection: Arc<AsyncConnection>,
    /// Maps request IDs to their response channels
    request_channels: Arc<RwLock<HashMap<i32, ChannelSender>>>,
    /// Maps shared channel types to their response channels
    shared_channels: Arc<RwLock<HashMap<OutgoingMessages, SharedChannelSender>>>,
    /// Maps order IDs to their response channels
    order_channels: Arc<RwLock<HashMap<i32, ChannelSender>>>,
    /// Optional channel for order update stream
    order_update_stream: Arc<RwLock<Option<ChannelSender>>>,
    /// Channel for cleanup signals
    cleanup_sender: mpsc::UnboundedSender<CleanupSignal>,
}

impl AsyncTcpMessageBus {
    /// Create a new async TCP message bus
    pub fn new(connection: AsyncConnection) -> Result<Self, Error> {
        let (cleanup_sender, cleanup_receiver) = mpsc::unbounded_channel();

        // Pre-create broadcast senders for all shared channels (like sync does)
        let mut shared_channels = HashMap::new();
        for mapping in shared_channel_configuration::CHANNEL_MAPPINGS {
            let (sender, _) = broadcast::channel(1000); // Buffer size for shared channels
            shared_channels.insert(mapping.request, sender);
        }

        let message_bus = Self {
            connection: Arc::new(connection),
            request_channels: Arc::new(RwLock::new(HashMap::new())),
            shared_channels: Arc::new(RwLock::new(shared_channels)),
            order_channels: Arc::new(RwLock::new(HashMap::new())),
            order_update_stream: Arc::new(RwLock::new(None)),
            cleanup_sender,
        };

        // Start cleanup task
        let request_channels = message_bus.request_channels.clone();
        let shared_channels = message_bus.shared_channels.clone();
        let order_channels = message_bus.order_channels.clone();

        task::spawn(async move {
            let mut receiver = cleanup_receiver;
            while let Some(signal) = receiver.recv().await {
                match signal {
                    CleanupSignal::Request(request_id) => {
                        let mut channels = request_channels.write().await;
                        channels.remove(&request_id);
                        debug!("Cleaned up request channel for ID: {request_id}");
                    }
                    CleanupSignal::Order(order_id) => {
                        let mut channels = order_channels.write().await;
                        channels.remove(&order_id);
                        debug!("Cleaned up order channel for ID: {order_id}");
                    }
                    CleanupSignal::Shared(message_type) => {
                        let mut channels = shared_channels.write().await;
                        channels.remove(&message_type);
                    }
                }
            }
        });

        Ok(message_bus)
    }

    /// Start processing messages from TWS
    pub fn process_messages(self: Arc<Self>, _server_version: i32, reconnect_delay: Duration) -> Result<(), Error> {
        let message_bus = self.clone();

        let _handle = task::spawn(async move {
            loop {
                match message_bus.read_and_route_message().await {
                    Ok(_) => continue,
                    Err(Error::ConnectionReset) => {
                        error!("Connection reset, attempting to reconnect...");
                        sleep(reconnect_delay).await;
                        // TODO: Implement reconnection logic
                        continue;
                    }
                    Err(e) => {
                        error!("Error processing message: {e}");
                        continue;
                    }
                }
            }
        });

        Ok(())
    }

    /// Read a message and route it to the appropriate channel
    async fn read_and_route_message(&self) -> Result<(), Error> {
        let message = self.connection.read_message().await?;

        // Use common routing logic
        match determine_routing(&message) {
            RoutingDecision::ByRequestId(request_id) => self.route_to_request_channel(request_id, message).await,
            RoutingDecision::ByOrderId(order_id) => self.route_to_order_channel(order_id, message).await,
            RoutingDecision::ByMessageType(message_type) => self.route_to_shared_channel(message_type, message).await,
            RoutingDecision::SharedMessage(message_type) => self.route_to_shared_channel(message_type, message).await,
            RoutingDecision::Error { request_id, error_code } => self.route_error_message_new(message, request_id, error_code).await,
        }
    }

    /// Route error message to appropriate channel
    #[allow(dead_code)]
    async fn route_error_message(&self, mut message: ResponseMessage) -> Result<(), Error> {
        message.skip(); // Skip message type
        message.skip(); // Skip version
        let request_id = message.next_int()?;
        let error_code = message.next_int()?;
        let error_msg = message.next_string()?;

        info!("Error message - Request ID: {request_id}, Code: {error_code}, Message: {error_msg}");

        // Route to request-specific channel if exists
        if request_id >= 0 {
            let channels = self.request_channels.read().await;
            if let Some(sender) = channels.get(&request_id) {
                let _ = sender.send(message);
            }
        }

        Ok(())
    }

    /// Route error message using routing decision
    async fn route_error_message_new(&self, message: ResponseMessage, request_id: i32, error_code: i32) -> Result<(), Error> {
        // Log the error for visibility
        let error_msg = if message.len() > 4 {
            message.peek_string(4)
        } else {
            String::from("Unknown error")
        };

        // Check if this is a warning or unspecified error
        if request_id == UNSPECIFIED_REQUEST_ID || is_warning_error(error_code) {
            // Log warnings differently
            if is_warning_error(error_code) {
                warn!("Warning - Request ID: {request_id}, Code: {error_code}, Message: {error_msg}");
            } else {
                error!("Error - Request ID: {request_id}, Code: {error_code}, Message: {error_msg}");
            }
        } else {
            // Route to request-specific channel
            info!("Error message - Request ID: {request_id}, Code: {error_code}, Message: {error_msg}");
            let channels = self.request_channels.read().await;
            if let Some(sender) = channels.get(&request_id) {
                let _ = sender.send(message);
            }
        }

        Ok(())
    }

    /// Route message to request-specific channel
    async fn route_to_request_channel(&self, request_id: i32, message: ResponseMessage) -> Result<(), Error> {
        let channels = self.request_channels.read().await;
        if let Some(sender) = channels.get(&request_id) {
            let _ = sender.send(message);
        }
        Ok(())
    }

    /// Route message to order-specific channel
    async fn route_to_order_channel(&self, order_id: i32, message: ResponseMessage) -> Result<(), Error> {
        // Send to order update stream if it exists
        self.send_order_update(&message).await;

        let channels = self.order_channels.read().await;
        if let Some(sender) = channels.get(&order_id) {
            let _ = sender.send(message);
        }
        Ok(())
    }

    /// Route message to shared channel
    async fn route_to_shared_channel(&self, message_type: IncomingMessages, message: ResponseMessage) -> Result<(), Error> {
        // Send order-related messages to order update stream
        match message_type {
            IncomingMessages::OpenOrder
            | IncomingMessages::OrderStatus
            | IncomingMessages::ExecutionData
            | IncomingMessages::CommissionsReport
            | IncomingMessages::CompletedOrder => {
                self.send_order_update(&message).await;
            }
            _ => {}
        }

        // Use the common mapping function to route to broadcast channel
        if let Some(channel_type) = map_incoming_to_outgoing(message_type) {
            let channels = self.shared_channels.read().await;
            if let Some(sender) = channels.get(&channel_type) {
                // Broadcast to all subscribers
                let _ = sender.send(message);
            }
        }

        Ok(())
    }

    /// Send message to order update stream if it exists
    async fn send_order_update(&self, message: &ResponseMessage) -> bool {
        let order_update_stream = self.order_update_stream.read().await;
        if let Some(sender) = order_update_stream.as_ref() {
            if let Err(e) = sender.send(message.clone()) {
                warn!("error sending to order update stream: {e}");
                return false;
            }
            return true;
        }
        false
    }
}

#[async_trait]
impl AsyncMessageBus for AsyncTcpMessageBus {
    async fn send_request(&self, request_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Create channel first
        let (sender, receiver) = mpsc::unbounded_channel();

        // Insert into map BEFORE sending
        {
            let mut channels = self.request_channels.write().await;
            channels.insert(request_id, sender);
        }

        // Now send the request - any response will find the channel
        self.connection.write_message(&message).await?;

        // Return subscription with cleanup
        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Request(request_id),
        ))
    }

    async fn send_order_request(&self, order_id: i32, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Same pattern for orders
        let (sender, receiver) = mpsc::unbounded_channel();

        {
            let mut channels = self.order_channels.write().await;
            channels.insert(order_id, sender);
        }

        self.connection.write_message(&message).await?;

        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Order(order_id),
        ))
    }

    async fn send_shared_request(&self, message_type: OutgoingMessages, message: RequestMessage) -> Result<AsyncInternalSubscription, Error> {
        // Get the pre-created broadcast sender and create a new receiver
        let receiver = {
            let channels = self.shared_channels.read().await;
            if let Some(sender) = channels.get(&message_type) {
                sender.subscribe()
            } else {
                return Err(Error::Simple(format!(
                    "No shared channel configured for message type: {:?}",
                    message_type
                )));
            }
        };

        // Send the request - response will be routed to the broadcast channel
        self.connection.write_message(&message).await?;

        // Convert broadcast receiver to mpsc for compatibility with AsyncInternalSubscription
        let (mpsc_sender, mpsc_receiver) = mpsc::unbounded_channel();

        // Spawn task to relay from broadcast to mpsc
        let mut broadcast_receiver = receiver;
        tokio::spawn(async move {
            while let Ok(message) = broadcast_receiver.recv().await {
                if mpsc_sender.send(message).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        Ok(AsyncInternalSubscription::with_cleanup(
            mpsc_receiver,
            self.cleanup_sender.clone(),
            CleanupSignal::Shared(message_type),
        ))
    }

    async fn send_message(&self, message: RequestMessage) -> Result<(), Error> {
        // For fire-and-forget messages
        self.connection.write_message(&message).await
    }

    async fn cancel_subscription(&self, request_id: i32, message: RequestMessage) -> Result<(), Error> {
        self.connection.write_message(&message).await?;

        let channels = self.request_channels.read().await;
        if let Some(sender) = channels.get(&request_id) {
            // Send cancellation error to the channel
            let _ = sender.send(ResponseMessage::from("Cancelled"));
        }

        // Remove channel
        let mut channels = self.request_channels.write().await;
        channels.remove(&request_id);

        Ok(())
    }

    async fn cancel_order_subscription(&self, order_id: i32, message: RequestMessage) -> Result<(), Error> {
        self.connection.write_message(&message).await?;

        let channels = self.order_channels.read().await;
        if let Some(sender) = channels.get(&order_id) {
            // Send cancellation error to the channel
            let _ = sender.send(ResponseMessage::from("Cancelled"));
        }

        // Remove channel
        let mut channels = self.order_channels.write().await;
        channels.remove(&order_id);

        Ok(())
    }

    async fn create_order_update_subscription(&self) -> Result<AsyncInternalSubscription, Error> {
        let mut order_update_stream = self.order_update_stream.write().await;

        if order_update_stream.is_some() {
            return Err(Error::AlreadySubscribed);
        }

        let (sender, receiver) = mpsc::unbounded_channel();

        *order_update_stream = Some(sender);

        Ok(AsyncInternalSubscription::new(receiver))
    }
}
