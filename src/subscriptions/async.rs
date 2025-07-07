//! Asynchronous subscription implementation

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::stream::Stream;
use tokio::sync::mpsc;

use super::common::{process_decode_result, ProcessingResult};
use crate::client::r#async::Client;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::transport::AsyncInternalSubscription;
use crate::Error;

/// Trait for types that can be decoded from response messages
pub trait AsyncDataStream<T> {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<T, Error>;
}

/// Asynchronous subscription for streaming data
pub struct Subscription<T> {
    inner: SubscriptionInner<T>,
}

enum SubscriptionInner<T> {
    /// Subscription with decoder - receives ResponseMessage and decodes to T
    WithDecoder {
        receiver: mpsc::UnboundedReceiver<ResponseMessage>,
        decoder: Box<dyn Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send>,
        client: Arc<Client>,
    },
    /// Pre-decoded subscription - receives T directly
    PreDecoded { receiver: mpsc::UnboundedReceiver<Result<T, Error>> },
}

impl<T> Subscription<T> {
    /// Create a subscription from an internal subscription and a decoder
    pub fn with_decoder<D>(internal: AsyncInternalSubscription, client: Arc<Client>, decoder: D) -> Self
    where
        D: Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send + 'static,
    {
        Self {
            inner: SubscriptionInner::WithDecoder {
                receiver: internal.receiver,
                decoder: Box::new(decoder),
                client,
            },
        }
    }

    /// Create a subscription from an internal subscription with a decoder function
    pub fn new_with_decoder<F>(internal: AsyncInternalSubscription, client: Arc<Client>, decoder: F) -> Self
    where
        F: Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send + 'static,
    {
        Self::with_decoder(internal, client, decoder)
    }

    /// Create a subscription from an internal subscription using the AsyncDataStream decoder
    pub fn new_from_internal<D>(internal: AsyncInternalSubscription, client: Arc<Client>) -> Self
    where
        D: AsyncDataStream<T> + 'static,
        T: 'static,
    {
        Self::with_decoder(internal, client, D::decode)
    }

    /// Create subscription from existing receiver (for backward compatibility)
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        // This creates a subscription that expects pre-decoded messages
        // Used for compatibility with existing code that manually decodes
        Self {
            inner: SubscriptionInner::PreDecoded { receiver },
        }
    }

    /// Get the next value from the subscription
    pub async fn next(&mut self) -> Option<Result<T, Error>>
    where
        T: 'static,
    {
        match &mut self.inner {
            SubscriptionInner::WithDecoder { receiver, decoder, client } => loop {
                match receiver.recv().await {
                    Some(mut message) => {
                        let result = decoder(client, &mut message);
                        match process_decode_result(result) {
                            ProcessingResult::Success(val) => return Some(Ok(val)),
                            ProcessingResult::EndOfStream => return None,
                            ProcessingResult::Retry => continue,
                            ProcessingResult::Error(err) => return Some(Err(err)),
                        }
                    }
                    None => return None,
                }
            },
            SubscriptionInner::PreDecoded { receiver } => receiver.recv().await,
        }
    }
}

impl<T: Unpin + 'static> Stream for Subscription<T> {
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match &mut this.inner {
            SubscriptionInner::WithDecoder { receiver, decoder, client } => {
                match receiver.poll_recv(cx) {
                    Poll::Ready(Some(mut message)) => {
                        let result = decoder(client, &mut message);
                        match process_decode_result(result) {
                            ProcessingResult::Success(val) => Poll::Ready(Some(Ok(val))),
                            ProcessingResult::EndOfStream => Poll::Ready(None),
                            ProcessingResult::Retry => {
                                // For retry, we need to re-poll
                                cx.waker().wake_by_ref();
                                Poll::Pending
                            }
                            ProcessingResult::Error(err) => Poll::Ready(Some(Err(err))),
                        }
                    }
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                }
            }
            SubscriptionInner::PreDecoded { receiver } => receiver.poll_recv(cx),
        }
    }
}
