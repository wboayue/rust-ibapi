//! Async stream abstraction for `AsyncConnection` / `AsyncTcpMessageBus`.
//!
//! Mirrors the sync `transport::sync::{Io, Reconnect, Stream}` triple, but
//! method-async via `#[async_trait]`. Frame-level: `read_message` returns the
//! already-unframed body so callers don't repeat the length-prefix dance.

use std::fmt;
use std::future::{poll_fn, Future};
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::warn;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{watch, Mutex, MutexGuard};

use crate::connection::common::Endpoint;
use crate::errors::Error;
use crate::transport::common::validate_frame_length;
use crate::transport::r#async::ShutdownSignal;
use crate::transport::raw_capture::RawFrameTap;

/// The caller can abandon the account operation until its first byte.
pub(crate) struct AccountAdmission(watch::Receiver<bool>);

pub(crate) struct AbandonAccountWrite(watch::Sender<bool>);

impl Drop for AbandonAccountWrite {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}

impl AccountAdmission {
    pub(crate) fn new() -> (Self, AbandonAccountWrite) {
        let (caller, receiver) = watch::channel(false);
        (Self(receiver), AbandonAccountWrite(caller))
    }

    pub(super) async fn cancelled(&self) {
        let mut receiver = self.0.clone();
        let _ = receiver.wait_for(|abandoned| *abandoned).await;
    }

    pub(crate) fn check(&self) -> Result<(), Error> {
        self.open().map(drop)
    }

    fn open(&self) -> Result<watch::Ref<'_, bool>, Error> {
        let guard = self.0.borrow();
        if *guard {
            return Err(Error::Cancelled);
        }
        Ok(guard)
    }
}

#[async_trait]
pub(crate) trait AsyncIo {
    async fn read_message(&self) -> Result<Vec<u8>, Error>;
    /// Write `buf` whole. A caller dropped before any byte is written sends
    /// nothing. An unfinished write that handed any bytes to the writer
    /// breaks the connection, including an unfinished flush. A buffered
    /// writer may have sent only part of the frame to its peer.
    async fn write_all(&self, buf: &[u8]) -> Result<(), Error>;
    async fn write_all_tracked(&self, buf: &[u8], started: &AtomicBool, admission: Option<&AccountAdmission>) -> Result<(), Error> {
        if let Some(admission) = admission {
            admission.check()?;
        }
        self.write_all(buf).await?;
        started.store(true, Ordering::Relaxed);
        Ok(())
    }
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
    /// Abandon the current connection: pending and later reads and writes on
    /// it fail with [`Error::ConnectionReset`] until `reconnect` replaces it.
    /// Unlike [`AsyncIo::close`], not terminal. The dispatcher calls it on a
    /// lost connection, so a write stuck on a peer that stopped reading cannot
    /// hold up the reconnect (a TCP write has no timeout). The default does
    /// nothing, for in-memory streams, whose writes never block. Twin of the
    /// blocking `Reconnect::sever`.
    fn sever(&self) {}
}

pub(crate) trait AsyncStream: AsyncIo + AsyncReconnect + Send + Sync + 'static + std::fmt::Debug {}

/// Where an [`AsyncSocket`] stands. `Live` and `Broken` alternate: a write
/// cut off mid-frame or a `sever` breaks the socket, `reconnect` makes it live
/// again.
/// `Closed` is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SocketState {
    Live,
    Broken,
    Closed,
}

impl SocketState {
    /// The error I/O fails with in this state; `None` while live.
    fn error(self) -> Option<Error> {
        match self {
            Self::Live => None,
            Self::Broken => Some(Error::ConnectionReset),
            Self::Closed => Some(Error::Shutdown),
        }
    }
}

type Reader = Box<dyn AsyncRead + Send + Unpin>;
type Writer = Box<dyn AsyncWrite + Send + Unpin>;
type ConnectFuture = Pin<Box<dyn Future<Output = io::Result<(Reader, Writer)>> + Send>>;

/// Opens one byte stream to TWS, already split. [`AsyncSocket`] calls it to
/// connect and again on every reconnect.
#[derive(Clone)]
pub(crate) struct Connector(Arc<dyn Fn() -> ConnectFuture + Send + Sync>);

impl Connector {
    /// Wrap a caller's connect function. The stream is split with
    /// [`tokio::io::split`], so reads and writes can still run concurrently.
    pub(crate) fn from_stream<F, Fut, T>(connect: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = io::Result<T>> + Send + 'static,
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        Self(Arc::new(move || {
            let connecting = connect();
            Box::pin(async move {
                let (reader, writer) = tokio::io::split(connecting.await?);
                Ok((Box::new(reader) as Reader, Box::new(writer) as Writer))
            })
        }))
    }

    /// The built-in TCP connector. Unavailable on wasm, where tokio has no
    /// `net` without `tokio_unstable`.
    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn tcp(address: impl Into<String>, tcp_no_delay: bool) -> Self {
        let address: Arc<str> = address.into().into();
        Self(Arc::new(move || {
            let address = Arc::clone(&address);
            Box::pin(async move {
                let stream = tokio::net::TcpStream::connect(&*address).await?;
                stream.set_nodelay(tcp_no_delay)?;
                // Owned halves, not `tokio::io::split`: no lock between them.
                let (reader, writer) = stream.into_split();
                Ok((Box::new(reader) as Reader, Box::new(writer) as Writer))
            })
        }))
    }

    async fn connect(&self) -> io::Result<(Reader, Writer)> {
        (self.0)().await
    }
}

impl Endpoint {
    /// The connector that reaches this endpoint. On wasm there is no built-in
    /// TCP socket, so an `address` is refused.
    pub(crate) fn into_connector(self) -> Result<Connector, Error> {
        match self {
            #[cfg(not(target_family = "wasm"))]
            Self::Tcp { address, tcp_no_delay } => Ok(Connector::tcp(address, tcp_no_delay)),
            #[cfg(target_family = "wasm")]
            Self::Tcp { address, .. } => Err(Error::InvalidArgument(format!(
                "ClientBuilder: no built-in TCP socket on wasm to reach {address}; open the stream yourself with `connector`"
            ))),
            Self::Stream(connector) => Ok(connector),
        }
    }
}

/// Production async stream over whatever byte stream its [`Connector`] opens:
/// TCP by default, or a caller's (`ClientBuilder::connector`). Holds the split
/// halves behind `Mutex` so reads and writes can run concurrently from the
/// dispatcher task and the request senders. Framing, frame validation, raw
/// capture and the break/close state below apply to every stream alike.
pub(crate) struct AsyncSocket {
    /// `None` once `close` has released them.
    reader: Mutex<Option<Reader>>,
    writer: Mutex<Option<Writer>>,
    /// Anything but `Live` wakes pending reads and writes and turns away
    /// whoever acquires a half after it. `Broken` once a write stops mid-frame
    /// (see [`FrameProgress`]), so reads fail and the dispatcher reconnects,
    /// or once the dispatcher severs a lost connection; `Closed` wakes the I/O
    /// holding the locks so `close` can take them.
    state: watch::Sender<SocketState>,
    /// Opens the stream again on `reconnect`.
    connector: Connector,
    /// Byte-level capture of the inbound stream. Disabled unless
    /// `IBAPI_RAW_CAPTURE_DIR` is set; see [`RawFrameTap`].
    tap: RawFrameTap,
}

impl fmt::Debug for AsyncSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncSocket")
            .field("state", &*self.state.borrow())
            .finish_non_exhaustive()
    }
}

impl AsyncSocket {
    pub(crate) async fn connect(connector: Connector) -> Result<Self, Error> {
        let (reader, writer) = connector.connect().await?;
        Ok(Self {
            reader: Mutex::new(Some(reader)),
            writer: Mutex::new(Some(writer)),
            state: watch::Sender::new(SocketState::Live),
            connector,
            tap: RawFrameTap::from_env(),
        })
    }

    fn is_closed(&self) -> bool {
        *self.state.borrow() == SocketState::Closed
    }

    /// Move to `to`, unless closed: a cut-off write or a reconnect landing
    /// after `close` must not revive the socket.
    fn set_state(&self, to: SocketState) {
        self.state.send_if_modified(|state| {
            let changed = *state != SocketState::Closed && *state != to;
            if changed {
                *state = to;
            }
            changed
        });
    }

    /// Both halves, reader then writer. `close` and `reconnect` hold them
    /// together only through here, so they cannot deadlock on the order.
    async fn lock_halves(&self) -> (MutexGuard<'_, Option<Reader>>, MutexGuard<'_, Option<Writer>>) {
        let reader = self.reader.lock().await;
        let writer = self.writer.lock().await;
        (reader, writer)
    }

    /// The half behind a held lock, or the error the socket's state calls for.
    /// `interrupted` would catch that too, but only after taking the lock.
    fn open_half<'a, T>(&self, half: &'a mut Option<T>) -> Result<&'a mut T, Error> {
        if let Some(err) = self.state.borrow().error() {
            return Err(err);
        }
        half.as_mut().ok_or(Error::Shutdown)
    }

    /// Resolves once the socket is closed.
    async fn closed(&self) {
        let mut rx = self.state.subscribe();
        // `Err` is unreachable: `self` owns the sender.
        let _ = rx.wait_for(|state| *state == SocketState::Closed).await;
    }

    /// Resolves once the socket is closed or a cut-off write breaks it, with
    /// the error a pending read or write should fail with.
    async fn interrupted(&self) -> Error {
        let mut rx = self.state.subscribe();
        // `Err` is unreachable: `self` owns the sender.
        let state = rx.wait_for(|state| *state != SocketState::Live).await.map(|state| *state);
        state.ok().and_then(SocketState::error).unwrap_or(Error::Shutdown)
    }
}

/// Tracks bytes accepted by the writer before its flush succeeds.
struct FrameProgress<'a> {
    socket: &'a AsyncSocket,
    len: usize,
    /// The bytes not yet written; `write_all_buf` advances it as it goes,
    /// cancelled or not.
    remaining: &'a [u8],
    started: &'a AtomicBool,
    flushed: bool,
}

impl Drop for FrameProgress<'_> {
    fn drop(&mut self) {
        let written = self.len - self.remaining.len();
        if written > 0 {
            self.started.store(true, Ordering::Relaxed);
        }
        if written > 0 && !self.flushed {
            warn!("write stopped before flushing {written} of {} bytes; resetting the connection", self.len);
            self.socket.set_state(SocketState::Broken);
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
impl AsyncIo for AsyncSocket {
    async fn read_message(&self) -> Result<Vec<u8>, Error> {
        let mut reader = self.reader.lock().await;
        let reader = self.open_half(&mut reader)?;
        tokio::select! {
            biased;
            err = self.interrupted() => Err(err),
            read = read_framed_message(reader, &self.tap) => read,
        }
    }

    async fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        self.write_all_tracked(buf, &AtomicBool::new(false), None).await
    }

    async fn write_all_tracked(&self, buf: &[u8], started: &AtomicBool, admission: Option<&AccountAdmission>) -> Result<(), Error> {
        let mut writer = if let Some(admission) = admission {
            tokio::select! {
                biased;
                () = admission.cancelled() => return Err(Error::Cancelled),
                writer = self.writer.lock() => writer,
            }
        } else {
            self.writer.lock().await
        };
        let writer = self.open_half(&mut writer)?;
        // Declared after `writer`, so it drops first: a break lands while the
        // writer is still held, and cannot interleave with `reconnect`.
        let mut progress = FrameProgress {
            socket: self,
            len: buf.len(),
            remaining: buf,
            started,
            flushed: false,
        };
        // Biased: a break or close landing after `open_half` still fails
        // before the write sends a byte. Later, `close` wakes a backpressured
        // write this way.
        tokio::select! {
            biased;
            err = self.interrupted() => Err(err),
            result = async {
                if let Some(admission) = admission {
                    let first_write = writer.write_buf(&mut progress.remaining);
                    tokio::pin!(first_write);
                    let written = tokio::select! {
                        biased;
                        () = admission.cancelled() => return Err(Error::Cancelled),
                        result = poll_fn(|cx| {
                            let _caller = admission.open()?;
                            first_write.as_mut().poll(cx).map_err(Error::from)
                        }) => result?,
                    };
                    if written == 0 {
                        return Err(io::Error::from(io::ErrorKind::WriteZero).into());
                    }
                }
                writer.write_all_buf(&mut progress.remaining).await?;
                writer.flush().await?;
                progress.flushed = true;
                Ok(())
            } => result,
        }
    }

    async fn close(&self) {
        // Latch before locking: wakes the I/O holding the locks, and a
        // reconnect that installs new halves first still loses them below.
        self.set_state(SocketState::Closed);
        let (mut reader, mut writer) = self.lock_halves().await;
        reader.take();
        writer.take();
    }
}

#[async_trait]
impl AsyncReconnect for AsyncSocket {
    async fn reconnect(&self) -> Result<(), Error> {
        if self.is_closed() {
            return Err(Error::Shutdown);
        }
        // A caller's connector has no OS connect timeout behind it: `close`
        // must still end a connect that never resolves, or `disconnect` waits
        // on the dispatcher forever.
        let (new_reader, new_writer) = tokio::select! {
            biased;
            () = self.closed() => return Err(Error::Shutdown),
            opened = self.connector.connect() => opened?,
        };
        let (mut reader, mut writer) = self.lock_halves().await;
        if self.is_closed() {
            return Err(Error::Shutdown);
        }
        *reader = Some(new_reader);
        *writer = Some(new_writer);
        self.set_state(SocketState::Live);
        // One capture file per stream: splicing two of them would read back
        // as a desync at the seam that never happened.
        self.tap.start_new_segment();
        Ok(())
    }

    async fn sleep(&self, duration: Duration, shutdown: &ShutdownSignal) {
        shutdown.sleep(duration).await
    }

    /// `Broken` wakes a write pending on the writer, as a cut-off frame
    /// does; `reconnect` sets `Live` again.
    fn sever(&self) {
        self.set_state(SocketState::Broken);
    }
}

impl AsyncStream for AsyncSocket {}

#[cfg(test)]
#[path = "io_tests.rs"]
pub(super) mod tests;
