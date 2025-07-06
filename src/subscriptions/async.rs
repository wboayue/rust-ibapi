//! Asynchronous subscription implementation

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Stream, StreamExt};
use tokio::sync::mpsc;

use crate::messages::ResponseMessage;
use crate::transport::AsyncInternalSubscription;
use crate::Error;

// TODO: When implementing async subscriptions, use the common utilities:
// use super::common::{should_retry_error, should_store_error, process_decode_result, ProcessingResult};

/// Asynchronous subscription for streaming data
pub struct Subscription<T> {
    receiver: mpsc::UnboundedReceiver<Result<T, Error>>,
}

impl<T> Subscription<T> {
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        Self { receiver }
    }

    /// Create a subscription from an internal subscription
    /// This requires a decoder function to transform ResponseMessage -> T
    pub fn new_from_internal(mut internal: AsyncInternalSubscription) -> Self
    where
        T: Send + 'static,
    {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn a task to decode messages
        tokio::spawn(async move {
            while let Some(message) = internal.next().await {
                // TODO: This needs a proper decoder function
                // For now, just send an error
                let _ = sender.send(Err(Error::Simple("Decoder not implemented".to_string())));
            }
        });

        Self { receiver }
    }

    /// Create a subscription with a custom decoder function
    pub fn new_with_decoder<F>(mut internal: AsyncInternalSubscription, decoder: F) -> Self
    where
        T: Send + 'static,
        F: Fn(ResponseMessage) -> Result<T, Error> + Send + 'static,
    {
        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(message) = internal.next().await {
                let result = decoder(message);
                if sender.send(result).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        Self { receiver }
    }
}

impl<T> Stream for Subscription<T> {
    type Item = Result<T, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
