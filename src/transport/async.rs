//! Asynchronous transport implementation

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::Error;
use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};

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
