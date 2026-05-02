//! In-memory frame-level `Stream` for transport tests.
//!
//! Mirrors `transport/async_memory.rs` for the sync transport. Operates at the
//! frame level (one push = one `read_message`-returnable body), since
//! `Io::read_message` returns an already-unframed body — no byte-level waker
//! plumbing required.
//!
//! Distinct from `MockSocket` in `transport/sync/tests.rs`, which pairs writes
//! with scripted responses and asserts request bytes. `MemoryStream` is a
//! lower-level fixture: tests push response frames and read captured writes,
//! and do their own assertions.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Condvar, Mutex};

use crate::errors::Error;
use crate::transport::sync::{Io, Reconnect, Stream};

#[derive(Default, Debug)]
struct Inner {
    inbound: VecDeque<Vec<u8>>,
    outbound: Vec<u8>,
    closed: bool,
}

/// In-memory sync stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default, Debug)]
pub(crate) struct MemoryStream {
    inner: Arc<(Mutex<Inner>, Condvar)>,
}

impl MemoryStream {
    /// Append a single message body to the inbound queue. Wakes any blocked reader.
    pub fn push_inbound(&self, body: Vec<u8>) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().inbound.push_back(body);
        cv.notify_one();
    }

    /// Snapshot of every byte the consumer has written.
    pub fn captured(&self) -> Vec<u8> {
        let (mutex, _) = &*self.inner;
        mutex.lock().unwrap().outbound.clone()
    }

    /// Signal EOF. Subsequent `read_message` calls return `Error::Io(UnexpectedEof)`,
    /// matching `TcpSocket::read_message`'s behavior on a closed peer.
    pub fn close(&self) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().closed = true;
        cv.notify_all();
    }
}

impl Io for MemoryStream {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        let (mutex, cv) = &*self.inner;
        let mut guard = mutex.lock()?;
        loop {
            if let Some(body) = guard.inbound.pop_front() {
                return Ok(body);
            }
            if guard.closed {
                return Err(Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "MemoryStream closed")));
            }
            guard = cv.wait(guard)?;
        }
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let (mutex, _) = &*self.inner;
        mutex.lock()?.outbound.extend_from_slice(buf);
        Ok(())
    }
}

impl Reconnect for MemoryStream {
    fn reconnect(&self) -> Result<(), Error> {
        Ok(())
    }
    fn sleep(&self, _duration: std::time::Duration) {}
}

impl Stream for MemoryStream {}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
