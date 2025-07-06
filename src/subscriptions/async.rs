//! Asynchronous subscription implementation

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc;

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
}

impl<T> Stream for Subscription<T> {
    type Item = Result<T, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
