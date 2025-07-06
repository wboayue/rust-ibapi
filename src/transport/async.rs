//! Asynchronous transport implementation

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info};
use tokio::sync::{mpsc, RwLock};
use tokio::task;
use tokio::time::{sleep, Duration};

use crate::connection::r#async::AsyncConnection;
use crate::messages::{shared_channel_configuration, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::Error;

/// Asynchronous message bus trait
#[async_trait]
pub trait AsyncMessageBus: Send + Sync {
    async fn send_request(&self, request: RequestMessage) -> Result<(), Error>;
    async fn subscribe(&self, request_id: i32) -> AsyncInternalSubscription;
    async fn subscribe_shared(&self, channel_type: OutgoingMessages) -> AsyncInternalSubscription;
    async fn subscribe_order(&self, order_id: i32) -> AsyncInternalSubscription;
}

/// Internal subscription for async implementation
pub struct AsyncInternalSubscription {
    receiver: mpsc::UnboundedReceiver<ResponseMessage>,
}

impl AsyncInternalSubscription {
    pub fn new(receiver: mpsc::UnboundedReceiver<ResponseMessage>) -> Self {
        Self { receiver }
    }

    pub async fn next(&mut self) -> Option<ResponseMessage> {
        self.receiver.recv().await
    }
}

type ChannelSender = mpsc::UnboundedSender<ResponseMessage>;

/// Asynchronous TCP message bus implementation
pub struct AsyncTcpMessageBus {
    connection: Arc<AsyncConnection>,
    /// Maps request IDs to their response channels
    request_channels: Arc<RwLock<HashMap<i32, ChannelSender>>>,
    /// Maps shared channel types to their response channels
    shared_channels: Arc<RwLock<HashMap<OutgoingMessages, ChannelSender>>>,
    /// Maps order IDs to their response channels
    order_channels: Arc<RwLock<HashMap<i32, ChannelSender>>>,
    /// Maps incoming message types to their corresponding outgoing types
    incoming_to_outgoing: HashMap<IncomingMessages, OutgoingMessages>,
}

impl AsyncTcpMessageBus {
    /// Create a new async TCP message bus
    pub fn new(connection: AsyncConnection) -> Result<Self, Error> {
        // Build the incoming to outgoing message mapping
        let mut incoming_to_outgoing = HashMap::new();
        for mapping in shared_channel_configuration::CHANNEL_MAPPINGS {
            for &response in mapping.responses {
                incoming_to_outgoing.insert(response, mapping.request);
            }
        }

        Ok(Self {
            connection: Arc::new(connection),
            request_channels: Arc::new(RwLock::new(HashMap::new())),
            shared_channels: Arc::new(RwLock::new(HashMap::new())),
            order_channels: Arc::new(RwLock::new(HashMap::new())),
            incoming_to_outgoing,
        })
    }

    /// Start processing messages from TWS
    pub fn process_messages(self: Arc<Self>, server_version: i32, reconnect_delay: Duration) -> Result<(), Error> {
        let message_bus = self.clone();

        task::spawn(async move {
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
                        error!("Error processing message: {}", e);
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
        let message_type = message.message_type();

        debug!("Received message type: {:?}", message_type);

        // Route based on message type
        match message_type {
            IncomingMessages::NextValidId => {
                // This is handled during connection establishment
                Ok(())
            }
            IncomingMessages::Error => self.route_error_message(message).await,
            _ => {
                // Try to route to request-specific channel first using the built-in request_id method
                if let Some(request_id) = message.request_id() {
                    self.route_to_request_channel(request_id, message).await
                } else {
                    // Route to shared channel based on message type
                    self.route_to_shared_channel(message_type, message).await
                }
            }
        }
    }

    /// Route error message to appropriate channel
    async fn route_error_message(&self, mut message: ResponseMessage) -> Result<(), Error> {
        message.skip(); // Skip message type
        message.skip(); // Skip version
        let request_id = message.next_int()?;
        let error_code = message.next_int()?;
        let error_msg = message.next_string()?;

        info!("Error message - Request ID: {}, Code: {}, Message: {}", request_id, error_code, error_msg);

        // Route to request-specific channel if exists
        if request_id >= 0 {
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

    /// Route message to shared channel
    async fn route_to_shared_channel(&self, message_type: IncomingMessages, message: ResponseMessage) -> Result<(), Error> {
        // Use the pre-built mapping to find the corresponding outgoing type
        if let Some(&channel_type) = self.incoming_to_outgoing.get(&message_type) {
            let channels = self.shared_channels.read().await;
            if let Some(sender) = channels.get(&channel_type) {
                let _ = sender.send(message);
            }
        }

        Ok(())
    }

    /// Send a request to TWS
    async fn send_request_internal(&self, request: RequestMessage) -> Result<(), Error> {
        self.connection.write_message(&request).await
    }
}

#[async_trait]
impl AsyncMessageBus for AsyncTcpMessageBus {
    async fn send_request(&self, request: RequestMessage) -> Result<(), Error> {
        self.send_request_internal(request).await
    }

    async fn subscribe(&self, request_id: i32) -> AsyncInternalSubscription {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut channels = self.request_channels.write().await;
        channels.insert(request_id, sender);

        AsyncInternalSubscription::new(receiver)
    }

    async fn subscribe_shared(&self, channel_type: OutgoingMessages) -> AsyncInternalSubscription {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut channels = self.shared_channels.write().await;
        channels.insert(channel_type, sender);

        AsyncInternalSubscription::new(receiver)
    }

    async fn subscribe_order(&self, order_id: i32) -> AsyncInternalSubscription {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut channels = self.order_channels.write().await;
        channels.insert(order_id, sender);

        AsyncInternalSubscription::new(receiver)
    }
}
