//! Asynchronous subscription implementation

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc;

use crate::Error;

/// Asynchronous subscription for streaming data
pub struct AsyncSubscription<T> {
    receiver: mpsc::UnboundedReceiver<Result<T, Error>>,
}

impl<T> AsyncSubscription<T> {
    pub fn new(receiver: mpsc::UnboundedReceiver<Result<T, Error>>) -> Self {
        Self { receiver }
    }
}

impl<T> Stream for AsyncSubscription<T> {
    type Item = Result<T, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
