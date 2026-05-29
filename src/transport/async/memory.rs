//! In-memory frame-level `AsyncStream` for transport tests.
//!
//! Mirrors `transport/sync/memory.rs` for the async transport. Operates at the
//! frame level: `read_message` returns one queued body per call.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;

use super::io::{AsyncIo, AsyncReconnect, AsyncStream};
use crate::errors::Error;

#[derive(Default)]
struct Inner {
    inbound: VecDeque<Vec<u8>>,
    outbound: Vec<u8>,
    closed: bool,
    /// Remaining `reconnect()` calls that should fail before one succeeds.
    reconnect_failures: usize,
}

/// In-memory async stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default)]
pub(crate) struct MemoryStream {
    inner: Arc<Mutex<Inner>>,
    notify: Arc<Notify>,
}

impl MemoryStream {
    /// Append a single message body to the inbound queue. Wakes any blocked reader.
    pub fn push_inbound(&self, body: Vec<u8>) {
        self.inner.lock().unwrap().inbound.push_back(body);
        self.notify.notify_one();
    }

    /// Snapshot of every byte the consumer has written.
    pub fn captured(&self) -> Vec<u8> {
        self.inner.lock().unwrap().outbound.clone()
    }

    /// Signal EOF. Subsequent `read_message` calls return `Error::Io(UnexpectedEof)`,
    /// matching `AsyncTcpSocket::read_message`'s behavior on a closed peer.
    pub fn close(&self) {
        self.inner.lock().unwrap().closed = true;
        self.notify.notify_waiters();
    }

    /// Schedule the next `count` `reconnect()` calls to fail with
    /// `Error::Simple`; subsequent calls succeed.
    pub fn set_reconnect_failures(&self, count: usize) {
        self.inner.lock().unwrap().reconnect_failures = count;
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
                let mut inner = self.inner.lock().unwrap();
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
        self.inner.lock().unwrap().outbound.extend_from_slice(buf);
        Ok(())
    }
}

#[async_trait]
impl AsyncReconnect for MemoryStream {
    async fn reconnect(&self) -> Result<(), Error> {
        // Scope the std::sync MutexGuard so it cannot be held across a future .await.
        let should_fail = {
            let mut inner = self.inner.lock().unwrap();
            if inner.reconnect_failures > 0 {
                inner.reconnect_failures -= 1;
                true
            } else {
                false
            }
        };
        if should_fail {
            Err(Error::Simple("simulated reconnect failure".into()))
        } else {
            Ok(())
        }
    }
    async fn sleep(&self, _duration: Duration) {}
}

impl AsyncStream for MemoryStream {}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
