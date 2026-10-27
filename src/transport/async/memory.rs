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
    /// `None` is a queued hang-up: the read that reaches it returns EOF, as
    /// from a peer that closed this connection.
    inbound: VecDeque<Option<Vec<u8>>>,
    outbound: Vec<u8>,
    hung_up: bool,
    /// Set by `AsyncIo::close`; unlike `hung_up`, `reopen` keeps it.
    retired: bool,
    /// Remaining `reconnect()` calls that should fail before one succeeds.
    reconnect_failures: usize,
}

/// In-memory async stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default)]
pub(crate) struct MemoryStream {
    inner: Arc<Mutex<Inner>>,
    notify: Arc<Notify>,
}

impl Inner {
    /// A new connection doesn't inherit the old one's hang-up: drop a queued
    /// hang-up the old connection stopped reading just short of.
    fn end_connection(&mut self) {
        while matches!(self.inbound.front(), Some(None)) {
            self.inbound.pop_front();
        }
    }
}

impl MemoryStream {
    /// Append a single message body to the inbound queue. Wakes any blocked reader.
    pub fn push_inbound(&self, body: Vec<u8>) {
        self.inner.lock().unwrap().inbound.push_back(Some(body));
        self.notify.notify_one();
    }

    /// Queue a hang-up after the messages already pushed: the read that reaches
    /// it returns `Error::Io(UnexpectedEof)`, like a peer closing after its
    /// last frame. Unlike [`hang_up`](Self::hang_up), it ends one connection:
    /// if the reader stops right before it, a reconnect discards it, so the
    /// next connection starts at the frame pushed after it.
    pub fn push_hang_up(&self) {
        self.inner.lock().unwrap().inbound.push_back(None);
        self.notify.notify_one();
    }

    /// Snapshot of every byte the consumer has written.
    pub fn captured(&self) -> Vec<u8> {
        self.inner.lock().unwrap().outbound.clone()
    }

    /// Signal EOF. Subsequent `read_message` calls return `Error::Io(UnexpectedEof)`,
    /// matching `AsyncTcpSocket::read_message`'s behavior on a closed peer.
    pub fn hang_up(&self) {
        self.inner.lock().unwrap().hung_up = true;
        self.notify.notify_waiters();
    }

    /// Undo a `hang_up`, so a stream broken to trigger a reconnect can serve
    /// the replayed handshake and stay readable afterwards.
    pub fn reopen(&self) {
        self.inner.lock().unwrap().hung_up = false;
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
                if inner.retired {
                    return Err(Error::Shutdown);
                }
                match inner.inbound.pop_front() {
                    Some(Some(body)) => return Ok(body),
                    Some(None) => return Err(closed()),
                    None if inner.hung_up => return Err(closed()),
                    None => {}
                }
            }

            notified.await;
        }
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        if inner.retired {
            return Err(Error::Shutdown);
        }
        inner.outbound.extend_from_slice(buf);
        Ok(())
    }

    /// Terminal, like TCP: unlike `hang_up`, `reopen` can't undo
    /// it. Same as the sync `MemoryStream`'s `Io::close`.
    async fn close(&self) {
        self.inner.lock().unwrap().retired = true;
        self.notify.notify_waiters();
    }
}

#[async_trait]
impl AsyncReconnect for MemoryStream {
    async fn reconnect(&self) -> Result<(), Error> {
        // Scope the std::sync MutexGuard so it cannot be held across a future .await.
        let should_fail = {
            let mut inner = self.inner.lock().unwrap();
            if inner.retired {
                return Err(Error::Shutdown);
            }
            if inner.reconnect_failures > 0 {
                inner.reconnect_failures -= 1;
                true
            } else {
                inner.end_connection();
                false
            }
        };
        if should_fail {
            Err(Error::Simple("simulated reconnect failure".into()))
        } else {
            Ok(())
        }
    }
    async fn sleep(&self, _duration: Duration, _shutdown: &crate::transport::r#async::ShutdownSignal) {}
}

impl AsyncStream for MemoryStream {}

/// The error `read_message` returns on a closed peer.
fn closed() -> Error {
    Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "MemoryStream closed"))
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
