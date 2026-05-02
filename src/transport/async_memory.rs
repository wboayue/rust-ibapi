//! In-memory `AsyncRead` + `AsyncWrite` stream for transport tests.
//!
//! Spike for the eliminate-mock-gateway plan (PR 1). Validates that a custom
//! `AsyncRead`/`AsyncWrite` impl with explicit waker registration round-trips
//! length-prefixed frames without busy-spinning. Not yet wired into
//! `AsyncConnection`; PR 2 does that.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Default)]
struct Inner {
    inbound: VecDeque<u8>,
    outbound: Vec<u8>,
    read_waker: Option<Waker>,
    closed: bool,
}

/// In-memory async stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default)]
pub(crate) struct MemoryStream {
    inner: Arc<Mutex<Inner>>,
}

impl MemoryStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append bytes that the consumer (production code) will read.
    pub fn push_inbound(&self, bytes: &[u8]) {
        let waker = {
            let mut inner = self.inner.lock().unwrap();
            inner.inbound.extend(bytes);
            inner.read_waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    /// Append a length-prefixed frame: 4-byte BE length + payload.
    pub fn push_frame(&self, payload: &[u8]) {
        let mut buf = Vec::with_capacity(4 + payload.len());
        buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(payload);
        self.push_inbound(&buf);
    }

    /// Snapshot of every byte the consumer has written.
    pub fn captured(&self) -> Vec<u8> {
        self.inner.lock().unwrap().outbound.clone()
    }

    /// Signal EOF. Subsequent `poll_read` calls return `Ok(())` with no bytes filled.
    pub fn close(&self) {
        let waker = {
            let mut inner = self.inner.lock().unwrap();
            inner.closed = true;
            inner.read_waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }
}

impl AsyncRead for MemoryStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.inbound.is_empty() {
            if inner.closed {
                return Poll::Ready(Ok(()));
            }
            inner.read_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let want = buf.remaining();
        let (a, b) = inner.inbound.as_slices();
        let from_a = a.len().min(want);
        buf.put_slice(&a[..from_a]);
        let mut consumed = from_a;
        if from_a < want {
            let from_b = b.len().min(want - from_a);
            buf.put_slice(&b[..from_b]);
            consumed += from_b;
        }
        inner.inbound.drain(..consumed);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MemoryStream {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.inner.lock().unwrap().outbound.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
#[path = "async_memory_tests.rs"]
mod tests;
