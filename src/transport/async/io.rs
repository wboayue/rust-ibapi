//! Async stream abstraction for `AsyncConnection` / `AsyncTcpMessageBus`.
//!
//! Mirrors the sync `transport::sync::{Io, Reconnect, Stream}` triple, but
//! method-async via `#[async_trait]`. Frame-level: `read_message` returns the
//! already-unframed body so callers don't repeat the length-prefix dance.

use std::time::Duration;

use async_trait::async_trait;
use log::warn;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{watch, Mutex};

use crate::errors::Error;
use crate::transport::common::validate_frame_length;
use crate::transport::r#async::ShutdownSignal;
use crate::transport::raw_capture::RawFrameTap;

#[async_trait]
pub(crate) trait AsyncIo {
    async fn read_message(&self) -> Result<Vec<u8>, Error>;
    /// Write `buf` whole. A caller dropped before any byte is written sends
    /// nothing. A write that stops partway (its caller dropped, or a failure)
    /// leaves the connection unusable, since TWS would read any later frame
    /// from the middle of the cut-off one: later writes must fail, and reads
    /// must fail with a connection-lost error (pending ones included), because
    /// only a read error makes the dispatcher reconnect. `reconnect` clears it.
    async fn write_all(&self, buf: &[u8]) -> Result<(), Error>;
    /// Release the physical stream, even while owners still hold it: pending
    /// and later reads and writes fail with [`Error::Shutdown`], and
    /// `reconnect` refuses. Idempotent.
    async fn close(&self);
}

#[async_trait]
pub(crate) trait AsyncReconnect {
    /// Replace the connection, clearing a break left by a cut-off write (see
    /// [`AsyncIo::write_all`]).
    async fn reconnect(&self) -> Result<(), Error>;
    /// Wait out the reconnect backoff, returning early once `shutdown` is
    /// requested. In-memory test streams return immediately.
    async fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal);
}

pub(crate) trait AsyncStream: AsyncIo + AsyncReconnect + Send + Sync + 'static + std::fmt::Debug {}

/// Production async stream over `tokio::net::TcpStream`. Holds the split halves
/// behind `Mutex` so reads and writes can run concurrently from the dispatcher
/// task and the request senders.
#[derive(Debug)]
pub(crate) struct AsyncTcpSocket {
    /// `None` once `close` has released them.
    reader: Mutex<Option<OwnedReadHalf>>,
    writer: Mutex<Option<OwnedWriteHalf>>,
    /// Latched by `close`. Wakes pending reads and writes so `close` can take
    /// their locks, and turns away whoever acquires a half after it.
    closed: watch::Sender<bool>,
    /// True once a write stops mid-frame (see [`FrameProgress`]); cleared by
    /// `reconnect`. Writes refuse and reads fail while it is set, so the
    /// dispatcher reconnects.
    broken: watch::Sender<bool>,
    connection_url: String,
    tcp_no_delay: bool,
    /// Byte-level capture of the inbound stream. Disabled unless
    /// `IBAPI_RAW_CAPTURE_DIR` is set; see [`RawFrameTap`].
    tap: RawFrameTap,
}

impl AsyncTcpSocket {
    pub async fn connect(address: &str, tcp_no_delay: bool) -> Result<Self, Error> {
        let stream = TcpStream::connect(address).await?;
        stream.set_nodelay(tcp_no_delay)?;
        let (read_half, write_half) = stream.into_split();
        Ok(Self {
            reader: Mutex::new(Some(read_half)),
            writer: Mutex::new(Some(write_half)),
            closed: watch::Sender::new(false),
            broken: watch::Sender::new(false),
            connection_url: address.to_string(),
            tcp_no_delay,
            tap: RawFrameTap::from_env(),
        })
    }

    fn is_broken(&self) -> bool {
        *self.broken.borrow()
    }

    fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }

    /// The half behind a held lock, or [`Error::Shutdown`] once closed.
    fn live<'a, T>(&self, half: &'a mut Option<T>) -> Result<&'a mut T, Error> {
        half.as_mut().filter(|_| !self.is_closed()).ok_or(Error::Shutdown)
    }

    /// Resolves once the socket is closed or a cut-off write breaks it, with
    /// the error a pending read or write should fail with.
    async fn interrupted(&self) -> Error {
        let mut closed = self.closed.subscribe();
        let mut broken = self.broken.subscribe();
        tokio::select! {
            biased;
            _ = closed.wait_for(|closed| *closed) => Error::Shutdown,
            _ = broken.wait_for(|broken| *broken) => Error::ConnectionReset,
        }
    }
}

/// How much of a frame `write_all` has handed to the socket. Dropped partway,
/// with its caller cancelled or after a failed write, it breaks the connection:
/// the wire now ends mid-frame, and the next frame would be misread from there.
struct FrameProgress<'a> {
    socket: &'a AsyncTcpSocket,
    len: usize,
    /// The bytes not yet written; `write_all_buf` advances it as it goes,
    /// cancelled or not.
    remaining: &'a [u8],
}

impl Drop for FrameProgress<'_> {
    fn drop(&mut self) {
        let written = self.len - self.remaining.len();
        if written > 0 && !self.remaining.is_empty() {
            warn!("write stopped after {written} of {} bytes; resetting the connection", self.len);
            self.socket.broken.send_replace(true);
        }
    }
}

/// Unframe one message, taping the raw bytes on the way past.
///
/// The tap sees the length prefix *before* [`validate_frame_length`] can reject
/// it, because a prefix that fails validation is precisely the artifact a
/// framing desync leaves behind. Mirrors the blocking
/// [`transport::sync::read_message`](crate::transport::sync::read_message).
pub(crate) async fn read_framed_message<R>(reader: &mut R, tap: &RawFrameTap) -> Result<Vec<u8>, Error>
where
    R: AsyncRead + Unpin + Send + ?Sized,
{
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await?;
    tap.record_length_prefix(&length_bytes);
    let message_length = validate_frame_length(u32::from_be_bytes(length_bytes) as usize)?;
    let mut data = vec![0u8; message_length];
    reader.read_exact(&mut data).await?;
    tap.record_body(&data);
    Ok(data)
}

#[async_trait]
impl AsyncIo for AsyncTcpSocket {
    async fn read_message(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.lock().await;
        let reader = self.live(&mut reader)?;
        tokio::select! {
            biased;
            err = self.interrupted() => Err(err),
            read = read_framed_message(reader, &self.tap) => read,
        }
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut writer = self.writer.lock().await;
        let writer = self.live(&mut writer)?;
        if self.is_broken() {
            return Err(Error::ConnectionReset);
        }
        // Declared after `writer`, so it drops first: a break lands while the
        // writer is still held, and cannot interleave with `reconnect`.
        let mut progress = FrameProgress {
            socket: self,
            len: buf.len(),
            remaining: buf,
        };
        // A backpressured write holds the lock; `close` wakes it to take over.
        tokio::select! {
            biased;
            err = self.interrupted() => Err(err),
            result = async {
                writer.write_all_buf(&mut progress.remaining).await?;
                writer.flush().await?;
                Ok(())
            } => result,
        }
    }

    async fn close(&self) {
        // Latch before locking: wakes the I/O holding the locks, and a
        // reconnect that installs new halves first still loses them below.
        self.closed.send_replace(true);
        let mut reader = self.reader.lock().await;
        let mut writer = self.writer.lock().await;
        reader.take();
        writer.take();
    }
}

#[async_trait]
impl AsyncReconnect for AsyncTcpSocket {
    async fn reconnect(&self) -> Result<(), Error> {
        if self.is_closed() {
            return Err(Error::Shutdown);
        }
        let stream = TcpStream::connect(&self.connection_url).await?;
        stream.set_nodelay(self.tcp_no_delay)?;
        let (new_reader, new_writer) = stream.into_split();
        let mut reader = self.reader.lock().await;
        let mut writer = self.writer.lock().await;
        if self.is_closed() {
            return Err(Error::Shutdown);
        }
        *reader = Some(new_reader);
        *writer = Some(new_writer);
        self.broken.send_replace(false);
        // One capture file per TCP stream: splicing two of them would read back
        // as a desync at the seam that never happened.
        self.tap.start_new_segment();
        Ok(())
    }

    async fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        shutdown.sleep(duration).await
    }
}

impl AsyncStream for AsyncTcpSocket {}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
