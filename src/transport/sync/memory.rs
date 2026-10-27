//! In-memory frame-level `Stream` for transport tests.
//!
//! Mirrors `transport/async/memory.rs` for the sync transport. Operates at the
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
    /// `None` is a queued hang-up: the read that reaches it returns EOF, as
    /// from a peer that closed this connection.
    inbound: VecDeque<Option<Vec<u8>>>,
    outbound: Vec<u8>,
    hung_up: bool,
    /// Set by `Io::close`; unlike `hung_up`, `reopen` keeps it.
    retired: bool,
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

/// In-memory sync stream. Cloning yields another handle to the same shared queues.
#[derive(Clone, Default, Debug)]
pub(crate) struct MemoryStream {
    inner: Arc<(Mutex<Inner>, Condvar)>,
}

impl MemoryStream {
    /// Append a single message body to the inbound queue. Wakes any blocked reader.
    pub fn push_inbound(&self, body: Vec<u8>) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().inbound.push_back(Some(body));
        cv.notify_one();
    }

    /// Queue a hang-up after the messages already pushed: the read that reaches
    /// it returns `Error::Io(UnexpectedEof)`, like a peer closing after its
    /// last frame. Unlike [`hang_up`](Self::hang_up), it ends one connection:
    /// if the reader stops right before it, a reconnect discards it, so the
    /// next connection starts at the frame pushed after it.
    pub fn push_hang_up(&self) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().inbound.push_back(None);
        cv.notify_one();
    }

    /// Snapshot of every byte the consumer has written.
    pub fn captured(&self) -> Vec<u8> {
        let (mutex, _) = &*self.inner;
        mutex.lock().unwrap().outbound.clone()
    }

    /// Signal EOF. Subsequent `read_message` calls return `Error::Io(UnexpectedEof)`,
    /// matching `TcpSocket::read_message`'s behavior on a closed peer.
    pub fn hang_up(&self) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().hung_up = true;
        cv.notify_all();
    }

    /// Undo a `hang_up`, so a stream broken to trigger a reconnect can serve
    /// the replayed handshake and stay readable afterwards.
    pub fn reopen(&self) {
        let (mutex, _) = &*self.inner;
        mutex.lock().unwrap().hung_up = false;
    }
}

impl Io for MemoryStream {
    fn read_message(&self) -> Result<Vec<u8>, Error> {
        let (mutex, cv) = &*self.inner;
        let mut guard = mutex.lock()?;
        loop {
            if guard.retired {
                return Err(Error::Shutdown);
            }
            match guard.inbound.pop_front() {
                Some(Some(body)) => return Ok(body),
                Some(None) => return Err(closed()),
                None if guard.hung_up => return Err(closed()),
                None => {}
            }
            guard = cv.wait(guard)?;
        }
    }

    fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let (mutex, _) = &*self.inner;
        let mut inner = mutex.lock()?;
        if inner.retired {
            return Err(Error::Shutdown);
        }
        inner.outbound.extend_from_slice(buf);
        Ok(())
    }

    /// Terminal, like TCP: unlike `hang_up`, `reopen` can't undo
    /// it. Same as the async `MemoryStream`'s `AsyncIo::close`.
    fn close(&self) {
        let (mutex, cv) = &*self.inner;
        mutex.lock().unwrap().retired = true;
        cv.notify_all();
    }
}

impl Reconnect for MemoryStream {
    fn reconnect(&self) -> Result<(), Error> {
        let (mutex, _) = &*self.inner;
        let mut inner = mutex.lock()?;
        if inner.retired {
            return Err(Error::Shutdown);
        }
        inner.end_connection();
        Ok(())
    }
    fn sleep(&self, _duration: std::time::Duration, _shutdown: &super::ShutdownSignal) {}
}

impl Stream for MemoryStream {}

/// The error `read_message` returns on a closed peer.
fn closed() -> Error {
    Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "MemoryStream closed"))
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
