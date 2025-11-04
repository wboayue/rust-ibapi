//! Asynchronous transport implementation

use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task;
use tokio::time::Duration;

/// Default capacity for broadcast channels
/// This should be large enough to handle bursts of messages without lagging
const BROADCAST_CHANNEL_CAPACITY: usize = 1024;

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

use super::routing::{determine_routing, is_warning_error, RoutingDecision, UNSPECIFIED_REQUEST_ID};

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

    /// Ensure shutdown of the message bus
    #[allow(dead_code)]
    async fn ensure_shutdown(&self);

    /// Request shutdown synchronously (for use in Drop)
    fn request_shutdown_sync(&self);

    /// Returns true if the client is currently connected to TWS/IB Gateway
    fn is_connected(&self) -> bool;

    #[cfg(test)]
    fn request_messages(&self) -> Vec<RequestMessage> {
        vec![]
    }
}

/// Internal subscription for async implementation
pub struct AsyncInternalSubscription {
    pub(crate) receiver: broadcast::Receiver<ResponseMessage>,
    cleanup_sender: Option<mpsc::UnboundedSender<CleanupSignal>>,
    cleanup_signal: Option<CleanupSignal>,
    cleanup_sent: bool,
}

impl Clone for AsyncInternalSubscription {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.resubscribe(),
            cleanup_sender: self.cleanup_sender.clone(),
            cleanup_signal: self.cleanup_signal.clone(),
            cleanup_sent: false, // Each clone should handle its own cleanup
        }
    }
}

impl AsyncInternalSubscription {
    pub fn new(receiver: broadcast::Receiver<ResponseMessage>) -> Self {
        Self {
            receiver,
            cleanup_sender: None,
            cleanup_signal: None,
            cleanup_sent: false,
        }
    }

    pub fn with_cleanup(
        receiver: broadcast::Receiver<ResponseMessage>,
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

    pub async fn next(&mut self) -> Option<Result<ResponseMessage, Error>> {
        loop {
            match self.receiver.recv().await {
                Ok(msg) => return Some(Ok(msg)),
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // If we lagged, continue the loop to try again
                    continue;
                }
            }
        }
    }

    /// Extract the receiver for use in subscriptions (disables cleanup)
    pub fn take_receiver(mut self) -> broadcast::Receiver<ResponseMessage> {
        // Disable cleanup by clearing the cleanup info - the subscription will now own the receiver
        self.cleanup_sender = None;
        self.cleanup_signal = None;
        self.cleanup_sent = true; // Mark as sent to prevent Drop from sending

        // Create a dummy receiver to replace the original one
        let (dummy_sender, dummy_receiver) = broadcast::channel(1);
        drop(dummy_sender); // Close the channel immediately
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

type BroadcastSender = broadcast::Sender<ResponseMessage>;

/// Asynchronous TCP message bus implementation
pub struct AsyncTcpMessageBus {
    connection: Arc<AsyncConnection>,
    /// Maps request IDs to their response channels
    request_channels: Arc<RwLock<HashMap<i32, BroadcastSender>>>,
    /// Maps IncomingMessages to broadcast senders (like sync does)
    shared_channel_senders: Arc<RwLock<HashMap<IncomingMessages, Vec<BroadcastSender>>>>,
    /// Maps OutgoingMessages to receivers for client subscription
    shared_channel_receivers: Arc<RwLock<HashMap<OutgoingMessages, broadcast::Receiver<ResponseMessage>>>>,
    /// Maps order IDs to their response channels
    order_channels: Arc<RwLock<HashMap<i32, BroadcastSender>>>,
    /// Maps execution IDs to their response channels (for commission reports)
    execution_channels: Arc<RwLock<HashMap<String, BroadcastSender>>>,
    /// Optional channel for order update stream
    order_update_stream: Arc<RwLock<Option<BroadcastSender>>>,
    /// Channel for cleanup signals
    cleanup_sender: mpsc::UnboundedSender<CleanupSignal>,
    /// Handle to the message processing task
    process_task: Arc<RwLock<Option<task::JoinHandle<()>>>>,
    /// Shutdown flag
    shutdown_requested: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
}

impl Drop for AsyncTcpMessageBus {
    fn drop(&mut self) {
        debug!("dropping async tcp message bus");
        // Set the shutdown flag so the background task exits
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }
}

impl AsyncTcpMessageBus {
    /// Create a new async TCP message bus
    pub fn new(connection: AsyncConnection) -> Result<Self, Error> {
        let (cleanup_sender, cleanup_receiver) = mpsc::unbounded_channel();

        // Pre-create broadcast channels for all shared channels (like sync does)
        let mut shared_channel_senders = HashMap::new();
        let mut shared_channel_receivers = HashMap::new();

        for mapping in shared_channel_configuration::CHANNEL_MAPPINGS {
            let (sender, receiver) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
            shared_channel_receivers.insert(mapping.request, receiver);

            // Map each response type to the sender (multiple response types can share same sender)
            for response_type in mapping.responses {
                shared_channel_senders.entry(*response_type).or_insert_with(Vec::new).push(sender.clone());
            }
        }

        let message_bus = Self {
            connection: Arc::new(connection),
            request_channels: Arc::new(RwLock::new(HashMap::new())),
            shared_channel_senders: Arc::new(RwLock::new(shared_channel_senders)),
            shared_channel_receivers: Arc::new(RwLock::new(shared_channel_receivers)),
            order_channels: Arc::new(RwLock::new(HashMap::new())),
            execution_channels: Arc::new(RwLock::new(HashMap::new())),
            order_update_stream: Arc::new(RwLock::new(None)),
            cleanup_sender,
            process_task: Arc::new(RwLock::new(None)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            connected: Arc::new(AtomicBool::new(true)),
        };

        // Start cleanup task
        let request_channels = message_bus.request_channels.clone();
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
                        // Shared channels are persistent and should not be removed
                        // They are created at initialization and reused across multiple requests
                        debug!("Subscription for shared channel {:?} ended (channel remains active)", message_type);
                    }
                }
            }
        });

        Ok(message_bus)
    }

    /// Start processing messages from TWS
    pub fn process_messages(self: Arc<Self>, _server_version: i32, _reconnect_delay: Duration) -> Result<(), Error> {
        let message_bus = self.clone();
        let shutdown_flag = self.shutdown_requested.clone();

        let handle = task::spawn(async move {
            loop {
                // Check for shutdown request
                if shutdown_flag.load(Ordering::Relaxed) {
                    debug!("Shutdown requested, stopping message processing");
                    break;
                }

                // Use tokio::select to check shutdown flag while waiting for messages
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        // Check shutdown flag periodically
                        continue;
                    }
                    result = message_bus.read_and_route_message() => {
                        use crate::client::error_handler::{is_connection_error, is_timeout_error};

                        match result {
                            Ok(_) => continue,
                            Err(ref err) if is_timeout_error(err) => {
                                if message_bus.shutdown_requested.load(Ordering::Relaxed) {
                                    debug!("dispatcher task exiting");
                                    break;
                                }
                                continue;
                            }
                            Err(ref err) if is_connection_error(err) => {
                                error!("Connection error detected, attempting to reconnect: {err:?}");
                                message_bus.connected.store(false, Ordering::Relaxed);

                                match message_bus.connection.reconnect().await {
                                    Ok(_) => {
                                        info!("Successfully reconnected to TWS/Gateway");
                                        message_bus.connected.store(true, Ordering::Relaxed);
                                        message_bus.reset_channels().await;
                                    }
                                    Err(e) => {
                                        error!("Failed to reconnect to TWS/Gateway: {e:?}");
                                        message_bus.request_shutdown().await;
                                        break;
                                    }
                                }
                                continue;
                            }
                            Err(Error::Shutdown) => {
                                error!("Received shutdown signal, stopping message processing.");
                                break;
                            }
                            Err(err) => {
                                error!("Error processing message (shutting down): {err:?}");
                                message_bus.request_shutdown().await;
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Store the task handle
        let process_task = self.process_task.clone();
        tokio::spawn(async move {
            let mut task_guard = process_task.write().await;
            *task_guard = Some(handle);
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
            RoutingDecision::Error { request_id, error_code } => self.route_error_message(message, request_id, error_code).await,
            RoutingDecision::Shutdown => {
                debug!("Received shutdown message, calling request_shutdown");
                self.request_shutdown().await;
                Err(Error::Shutdown)
            }
        }
    }

    /// Reset all channels after reconnection
    async fn reset_channels(&self) {
        debug!("resetting message bus channels");

        {
            let channels = self.request_channels.read().await;
            for (_, sender) in channels.iter() {
                let error_msg = ResponseMessage::from("ConnectionReset");
                let _ = sender.send(error_msg);
            }
        }

        {
            let channels = self.order_channels.read().await;
            for (_, sender) in channels.iter() {
                let error_msg = ResponseMessage::from("ConnectionReset");
                let _ = sender.send(error_msg);
            }
        }

        {
            let mut channels = self.request_channels.write().await;
            channels.clear();
        }

        {
            let mut channels = self.order_channels.write().await;
            channels.clear();
        }

        {
            let mut channels = self.execution_channels.write().await;
            channels.clear();
        }
    }

    /// Notify all waiting subscriptions about shutdown
    async fn request_shutdown(&self) {
        debug!("shutdown requested");

        // Set the shutdown flag and mark as disconnected
        self.connected.store(false, Ordering::Relaxed);
        self.shutdown_requested.store(true, Ordering::Relaxed);

        // Clear all channels - dropping the senders will close the channels
        // and cause all receivers to get RecvError::Closed
        {
            let mut channels = self.request_channels.write().await;
            channels.clear();
        }

        {
            let mut channels = self.order_channels.write().await;
            channels.clear();
        }

        {
            let mut channels = self.shared_channel_senders.write().await;
            channels.clear();
        }

        {
            let mut channels = self.shared_channel_receivers.write().await;
            channels.clear();
        }

        {
            let mut order_update_stream = self.order_update_stream.write().await;
            *order_update_stream = None;
        }
    }

    /// Route error message using routing decision
    async fn route_error_message(&self, message: ResponseMessage, request_id: i32, error_code: i32) -> Result<(), Error> {
        let _ = self.send_order_update(&message).await;

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
            // Route to request-specific channel or order channel
            info!("Error message - Request ID: {request_id}, Code: {error_code}, Message: {error_msg}");

            // Check if this error is order-related (has an order_id)
            let sent_to_update_stream = if message.order_id().is_some() {
                self.send_order_update(&message).await
            } else {
                false
            };

            // First try request channels
            let channels = self.request_channels.read().await;
            if let Some(sender) = channels.get(&request_id) {
                let _ = sender.send(message);
            } else {
                // If not in request channels, try order channels (for cancel_order, etc.)
                let order_channels = self.order_channels.read().await;
                if let Some(sender) = order_channels.get(&request_id) {
                    let _ = sender.send(message.clone());
                } else if !sent_to_update_stream && message.order_id().is_some() {
                    info!("order error message has no recipient: {:?}", message);
                }
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
        let routed = self.send_order_update(&message).await;
        let message_type = message.message_type();

        // Special handling for different order message types
        match message_type {
            IncomingMessages::ExecutionData => {
                let order_id = message.order_id();
                let request_id = message.request_id();

                // First check matching orders channel
                if let Some(actual_order_id) = order_id {
                    let channels = self.order_channels.read().await;
                    if let Some(sender) = channels.get(&actual_order_id) {
                        // Store execution_id -> sender mapping for commission reports
                        if let Some(execution_id) = message.execution_id() {
                            let mut exec_channels = self.execution_channels.write().await;
                            exec_channels.insert(execution_id, sender.clone());
                        }
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }

                // Then check request channel (for executions() API calls)
                if let Some(req_id) = request_id {
                    let channels = self.request_channels.read().await;
                    if let Some(sender) = channels.get(&req_id) {
                        // Store execution_id -> sender mapping for commission reports
                        if let Some(execution_id) = message.execution_id() {
                            let mut exec_channels = self.execution_channels.write().await;
                            exec_channels.insert(execution_id, sender.clone());
                        }
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }

                if !routed {
                    warn!("could not route ExecutionData message {:?}", message);
                }
            }
            IncomingMessages::CommissionsReport => {
                // Route CommissionReport using execution_id
                if let Some(execution_id) = message.execution_id() {
                    let exec_channels = self.execution_channels.read().await;
                    if let Some(sender) = exec_channels.get(&execution_id) {
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }
                // Fall through to shared channel check
            }
            IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => {
                // For OpenOrder and OrderStatus, try order channel first
                if let Some(actual_order_id) = message.order_id() {
                    let channels = self.order_channels.read().await;
                    if let Some(sender) = channels.get(&actual_order_id) {
                        let _ = sender.send(message);
                        return Ok(());
                    }

                    // If no order channel, check if there's a shared channel (like sync does)
                    let shared_channels = self.shared_channel_senders.read().await;
                    if let Some(senders) = shared_channels.get(&message_type) {
                        for sender in senders {
                            if let Err(e) = sender.send(message.clone()) {
                                warn!("error sending to shared channel for {message_type:?}: {e}");
                            }
                        }
                        return Ok(());
                    }
                }

                if !routed {
                    warn!("could not route message {:?}", message);
                }
            }
            IncomingMessages::ExecutionDataEnd => {
                let order_id = message.order_id();
                let request_id = message.request_id();

                // First check matching orders channel
                if let Some(actual_order_id) = order_id {
                    let channels = self.order_channels.read().await;
                    if let Some(sender) = channels.get(&actual_order_id) {
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }

                // Then check request channel
                if let Some(req_id) = request_id {
                    let channels = self.request_channels.read().await;
                    if let Some(sender) = channels.get(&req_id) {
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }

                warn!("could not route ExecutionDataEnd message {:?}", message);
            }
            IncomingMessages::CompletedOrder | IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => {
                // These messages don't have order IDs, route to shared channel if available
                let shared_channels = self.shared_channel_senders.read().await;
                if let Some(senders) = shared_channels.get(&message_type) {
                    for sender in senders {
                        if let Err(e) = sender.send(message.clone()) {
                            warn!("error sending to shared channel for {message_type:?}: {e}");
                        }
                    }
                    return Ok(());
                }

                if !routed {
                    warn!("could not route message {:?}", message);
                }
            }
            _ => {
                // All other order messages route by order_id
                if order_id >= 0 {
                    let channels = self.order_channels.read().await;
                    if let Some(sender) = channels.get(&order_id) {
                        let _ = sender.send(message);
                        return Ok(());
                    }
                }

                if !routed {
                    warn!("could not route message {:?}", message);
                }
            }
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

        // Route to all senders for this message type (like sync does)
        let channels = self.shared_channel_senders.read().await;
        if let Some(senders) = channels.get(&message_type) {
            // Broadcast to all subscribers
            for sender in senders {
                if let Err(e) = sender.send(message.clone()) {
                    warn!("error sending to shared channel for {message_type:?}: {e}");
                }
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
        // Create broadcast channel with reasonable buffer
        let (sender, receiver) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);

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
        let (sender, receiver) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);

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
        // Get the pre-created broadcast receiver
        let receiver = {
            let channels = self.shared_channel_receivers.read().await;
            if let Some(receiver) = channels.get(&message_type) {
                receiver.resubscribe()
            } else {
                return Err(Error::Simple(format!(
                    "No shared channel configured for message type: {:?}",
                    message_type
                )));
            }
        };

        // Send the request - response will be routed to the broadcast channel
        self.connection.write_message(&message).await?;

        // Return subscription directly - no relay needed!
        Ok(AsyncInternalSubscription::with_cleanup(
            receiver,
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

        let (sender, receiver) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);

        *order_update_stream = Some(sender);

        Ok(AsyncInternalSubscription::new(receiver))
    }

    async fn ensure_shutdown(&self) {
        debug!("ensure_shutdown called");

        // Request shutdown
        self.request_shutdown().await;

        // Wait for the processing task to finish
        let task_handle = {
            let mut task_guard = self.process_task.write().await;
            task_guard.take()
        };

        if let Some(handle) = task_handle {
            debug!("Waiting for processing task to finish");
            if let Err(e) = handle.await {
                warn!("Error joining processing task: {e}");
            }
            debug!("Processing task finished");
        }
    }

    fn request_shutdown_sync(&self) {
        debug!("sync shutdown requested");
        self.connected.store(false, Ordering::Relaxed);
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed) && !self.shutdown_requested.load(Ordering::Relaxed)
    }
}
