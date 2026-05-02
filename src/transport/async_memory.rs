//! In-memory frame-level `AsyncStream` for transport tests.
//!
//! Mirrors `transport/sync/memory.rs` for the async transport. Operates at the
//! frame level: `read_message` returns one queued body per call. The earlier
//! byte-level `AsyncRead`/`AsyncWrite` design from PR 1 was replaced once the
//! `AsyncIo` / `AsyncReconnect` traits landed (PR 2c-prep) — `AsyncTcpSocket`
//! handles framing internally so the fixture no longer needs raw byte plumbing.

use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use super::io::{AsyncIo, AsyncReconnect, AsyncStream};
use crate::errors::Error;

#[derive(Default)]
struct Inner {
    inbound: VecDeque<Vec<u8>>,
    outbound: Vec<u8>,
    closed: bool,
}

/// In-memory async stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default)]
pub(crate) struct MemoryStream {
    inner: Arc<Mutex<Inner>>,
    notify: Arc<Notify>,
}

impl MemoryStream {
    /// Append a single message body to the inbound queue. Wakes any blocked reader.
    pub async fn push_inbound(&self, body: Vec<u8>) {
        self.inner.lock().await.inbound.push_back(body);
        self.notify.notify_one();
    }

    /// Snapshot of every byte the consumer has written.
    pub async fn captured(&self) -> Vec<u8> {
        self.inner.lock().await.outbound.clone()
    }

    /// Signal EOF. Subsequent `read_message` calls return `Error::Io(UnexpectedEof)`,
    /// matching `AsyncTcpSocket::read_message`'s behavior on a closed peer.
    pub async fn close(&self) {
        self.inner.lock().await.closed = true;
        self.notify.notify_waiters();
    }
}

impl std::fmt::Debug for MemoryStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryStream").finish_non_exhaustive()
    }
}

#[async_trait]
impl AsyncIo for MemoryStream {
    async fn read_message(&self) -> Result<Vec<u8>, Error> {
        loop {
            // Arm the notification BEFORE checking state, so a `notify_one`
            // racing with the check can't be lost.
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            {
                let mut inner = self.inner.lock().await;
                if let Some(body) = inner.inbound.pop_front() {
                    return Ok(body);
                }
                if inner.closed {
                    return Err(Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "MemoryStream closed")));
                }
            }

            notified.await;
        }
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        self.inner.lock().await.outbound.extend_from_slice(buf);
        Ok(())
    }
}

#[async_trait]
impl AsyncReconnect for MemoryStream {
    async fn reconnect(&self) -> Result<(), Error> {
        Ok(())
    }
    async fn sleep(&self, _duration: Duration) {}
}

impl AsyncStream for MemoryStream {}

#[cfg(test)]
#[path = "async_memory_tests.rs"]
mod tests;
