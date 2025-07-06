//! Asynchronous subscription implementation

use std::marker::PhantomData;
use std::pin::Pin;
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
    receiver: mpsc::UnboundedReceiver<Result<T, Error>>,
    _phantom: PhantomData<T>,
}

impl<T> Subscription<T> {
    /// Create a subscription from a receiver
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        Self {
            receiver,
            _phantom: PhantomData,
        }
    }

    /// Create a subscription from an internal subscription with a decoder
    pub fn new_with_decoder<D, F>(mut internal: AsyncInternalSubscription, decoder: F, client: Client) -> Self
    where
        T: Send + 'static,
        D: AsyncDataStream<T> + 'static,
        F: Fn(&Client, &mut ResponseMessage) -> Result<T, Error> + Send + 'static,
    {
        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(mut message) = internal.next().await {
                let result = decoder(&client, &mut message);
                match process_decode_result(result) {
                    ProcessingResult::Success(val) => {
                        if sender.send(Ok(val)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    ProcessingResult::EndOfStream => break,
                    ProcessingResult::Retry => continue,
                    ProcessingResult::Error(err) => {
                        if sender.send(Err(err)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }
            }
        });

        Self::new(receiver)
    }

    /// Create a subscription from an internal subscription using the AsyncDataStream decoder
    pub fn new_from_internal<D>(internal: AsyncInternalSubscription, client: Client) -> Self
    where
        T: Send + 'static,
        D: AsyncDataStream<T> + 'static,
    {
        Self::new_with_decoder::<D, _>(internal, D::decode, client)
    }

    /// Get the next value from the subscription
    pub async fn next(&mut self) -> Option<Result<T, Error>> {
        self.receiver.recv().await
    }
}

impl<T: Unpin> Stream for Subscription<T> {
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        this.receiver.poll_recv(cx)
    }
}
