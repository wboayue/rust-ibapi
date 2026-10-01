//! Async stream abstraction for `AsyncConnection` / `AsyncTcpMessageBus`.
//!
//! Mirrors the sync `transport::sync::{Io, Reconnect, Stream}` triple, but
//! method-async via `#[async_trait]`. Frame-level: `read_message` returns the
//! already-unframed body so callers don't repeat the length-prefix dance.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use log::warn;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};

use crate::errors::Error;
use crate::transport::common::validate_frame_length;
use crate::transport::r#async::ShutdownSignal;
use crate::transport::raw_capture::RawFrameTap;

#[async_trait]
pub(crate) trait AsyncIo {
    async fn read_message(&self) -> Result<Vec<u8>, Error>;
    /// Write `buf` whole. A caller dropped before any byte is written sends
    /// nothing. A write that stops partway (its caller dropped, or a failure)
    /// must not be followed by more frames on that connection, which TWS would
    /// read from the middle of the cut-off one.
    async fn write_all(&self, buf: &[u8]) -> Result<(), Error>;
}

#[async_trait]
pub(crate) trait AsyncReconnect {
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
    reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
    /// Set when a write stops mid-frame (see [`FrameProgress`]); cleared by
    /// `reconnect`. Writes refuse and reads fail while it is set, so the
    /// dispatcher reconnects.
    broken: AtomicBool,
    /// Wakes a pending `read_message` when `broken` is set.
    broken_notify: Notify,
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
            reader: Mutex::new(read_half),
            writer: Mutex::new(write_half),
            broken: AtomicBool::new(false),
            broken_notify: Notify::new(),
            connection_url: address.to_string(),
            tcp_no_delay,
            tap: RawFrameTap::from_env(),
        })
    }

    fn break_connection(&self) {
        self.broken.store(true, Ordering::Release);
        self.broken_notify.notify_waiters();
    }
}

/// How much of a frame `write_all` has handed to the socket. Dropped partway,
/// with its caller cancelled or after a failed write, it breaks the connection:
/// the wire now ends mid-frame, and the next frame would be misread from there.
struct FrameProgress<'a> {
    socket: &'a AsyncTcpSocket,
    written: usize,
    len: usize,
}

impl Drop for FrameProgress<'_> {
    fn drop(&mut self) {
        if self.written > 0 && self.written < self.len {
            warn!("write stopped after {} of {} bytes; resetting the connection", self.written, self.len);
            self.socket.break_connection();
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
        // Registered before the check, so a break in between still wakes it.
        let broken = self.broken_notify.notified();
        tokio::pin!(broken);
        broken.as_mut().enable();
        if self.broken.load(Ordering::Acquire) {
            return Err(Error::ConnectionReset);
        }
        tokio::select! {
            read = read_framed_message(&mut *reader, &self.tap) => read,
            _ = broken => Err(Error::ConnectionReset),
        }
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut writer = self.writer.lock().await;
        if self.broken.load(Ordering::Acquire) {
            return Err(Error::ConnectionReset);
        }
        // Declared after `writer`, so it drops first: a break lands while the
        // writer is still held, and cannot interleave with `reconnect`.
        let mut progress = FrameProgress {
            socket: self,
            written: 0,
            len: buf.len(),
        };
        while progress.written < buf.len() {
            match writer.write(&buf[progress.written..]).await? {
                0 => return Err(std::io::Error::from(std::io::ErrorKind::WriteZero).into()),
                n => progress.written += n,
            }
        }
        writer.flush().await?;
        Ok(())
    }
}

#[async_trait]
impl AsyncReconnect for AsyncTcpSocket {
    async fn reconnect(&self) -> Result<(), Error> {
        let stream = TcpStream::connect(&self.connection_url).await?;
        stream.set_nodelay(self.tcp_no_delay)?;
        let (new_reader, new_writer) = stream.into_split();
        *self.reader.lock().await = new_reader;
        {
            let mut writer = self.writer.lock().await;
            *writer = new_writer;
            self.broken.store(false, Ordering::Release);
        }
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
