//! Async stream abstraction for `AsyncConnection` / `AsyncTcpMessageBus`.
//!
//! Mirrors the sync `transport::sync::{Io, Reconnect, Stream}` triple, but
//! method-async via `#[async_trait]`. Frame-level: `read_message` returns the
//! already-unframed body so callers don't repeat the length-prefix dance.

use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::errors::Error;
use crate::transport::common::validate_frame_length;
use crate::transport::raw_capture::RawFrameTap;

#[async_trait]
pub(crate) trait AsyncIo {
    async fn read_message(&self) -> Result<Vec<u8>, Error>;
    async fn write_all(&self, buf: &[u8]) -> Result<(), Error>;
}

#[async_trait]
pub(crate) trait AsyncReconnect {
    async fn reconnect(&self) -> Result<(), Error>;
    async fn sleep(&self, duration: Duration);
}

pub(crate) trait AsyncStream: AsyncIo + AsyncReconnect + Send + Sync + 'static + std::fmt::Debug {}

/// Production async stream over `tokio::net::TcpStream`. Holds the split halves
/// behind `Mutex` so reads and writes can run concurrently from the dispatcher
/// task and the request senders.
#[derive(Debug)]
pub(crate) struct AsyncTcpSocket {
    reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
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
            connection_url: address.to_string(),
            tcp_no_delay,
            tap: RawFrameTap::from_env(),
        })
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
        read_framed_message(&mut *reader, &self.tap).await
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        let mut writer = self.writer.lock().await;
        writer.write_all(buf).await?;
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
        *self.writer.lock().await = new_writer;
        // One capture file per TCP stream: splicing two of them would read back
        // as a desync at the seam that never happened.
        self.tap.start_new_segment();
        Ok(())
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

impl AsyncStream for AsyncTcpSocket {}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
