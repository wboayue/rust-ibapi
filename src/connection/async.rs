//! Asynchronous connection implementation

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use log::{debug, info};
use tokio::sync::{broadcast, Mutex};

use super::common::{
    parse_handshake_ack, parse_raw_message, require_protobuf_support, AccountInfo, ConnectionHandler, ConnectionProtocol, StartupHandshakeContext,
    StartupMessage, MAX_ACCOUNT_INFO_ATTEMPTS,
};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_raw_length, Notice, ResponseMessage};
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::r#async::{AsyncStream, AsyncTcpSocket, NoticeBroadcaster, ShutdownSignal};
use crate::transport::recorder::MessageRecorder;

type Response = Result<ResponseMessage, Error>;

/// Asynchronous connection to TWS, generic over the underlying `AsyncStream`.
/// The default `AsyncTcpSocket` is the production wiring; tests can substitute
/// an in-memory stream to drive the bus deterministically.
pub struct AsyncConnection<S: AsyncStream = AsyncTcpSocket> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) server_version_cache: AtomicI32,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
    /// Optional typed-message callback supplied via [`ClientBuilder::startup_callback`].
    /// Fires on initial handshake *and* every auto-reconnect handshake.
    startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
    /// Fan-out for unrouted notices. Shared with the bus (the bus reads via
    /// `self.connection.notice_broadcaster`) and any pre-bound `NoticeStream` the
    /// user obtained from `ClientBuilder::connect_with_notice_stream`.
    pub(crate) notice_broadcaster: NoticeBroadcaster,
    /// Reconnection attempts before `reconnect` gives up; `None` retries
    /// forever. Defaults to `Some(`[`MAX_RECONNECT_ATTEMPTS`]`)`.
    max_reconnect_attempts: Option<u32>,
    /// Shared with the bus so `reconnect` can abandon its backoff as soon as
    /// shutdown is requested. See [`AsyncConnection::shutdown_signal`].
    shutdown: Arc<ShutdownSignal>,
}

impl<S: AsyncStream> std::fmt::Debug for AsyncConnection<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncConnection")
            .field("client_id", &self.client_id)
            .field("server_version_cache", &self.server_version_cache.load(Ordering::Acquire))
            .field("startup_callback", &self.startup_callback.is_some())
            .finish()
    }
}

impl AsyncConnection<AsyncTcpSocket> {
    /// Create a connection from explicit pieces handed in by the builder.
    ///
    /// `notice_sender` is shared with any pre-bound `NoticeStream`; the builder
    /// allocates the broadcast channel before calling here. Persists across
    /// reconnects.
    pub(crate) async fn with_pieces(
        address: &str,
        client_id: i32,
        tcp_no_delay: bool,
        startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
        notice_sender: broadcast::Sender<Notice>,
        max_reconnect_attempts: Option<u32>,
    ) -> Result<Self, Error> {
        let socket = AsyncTcpSocket::connect(address, tcp_no_delay).await?;
        let mut connection = Self::with_socket(socket, client_id, startup_callback, notice_sender);
        connection.max_reconnect_attempts = max_reconnect_attempts;
        connection.establish_connection().await?;
        Ok(connection)
    }
}

impl<S: AsyncStream> AsyncConnection<S> {
    pub(crate) fn with_socket(
        socket: S,
        client_id: i32,
        startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
        notice_sender: broadcast::Sender<Notice>,
    ) -> Self {
        Self {
            client_id,
            socket,
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            server_version_cache: AtomicI32::new(0),
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
            startup_callback,
            notice_broadcaster: NoticeBroadcaster::new(notice_sender),
            max_reconnect_attempts: Some(MAX_RECONNECT_ATTEMPTS),
            shutdown: Arc::new(ShutdownSignal::default()),
        }
    }

    /// Handle on the shutdown flag `reconnect` observes. The bus takes a
    /// clone and requests shutdown through it.
    pub(crate) fn shutdown_signal(&self) -> Arc<ShutdownSignal> {
        Arc::clone(&self.shutdown)
    }

    /// Build a connection over an arbitrary `AsyncStream` without performing
    /// the handshake. For tests; the production path uses `with_pieces` which
    /// runs `establish_connection` immediately after construction.
    #[cfg(test)]
    pub(crate) fn stubbed(socket: S, client_id: i32) -> Self {
        let (notice_sender, _) = broadcast::channel(crate::transport::r#async::BROADCAST_CHANNEL_CAPACITY);
        Self::with_socket(socket, client_id, None, notice_sender)
    }

    /// Pin a post-handshake server version on a stubbed connection so
    /// `parse_raw_message` sees frames in the binary-text-payload / proto
    /// regime without going through `establish_connection`.
    #[cfg(test)]
    pub(crate) fn set_server_version_for_test(&self, server_version: i32) {
        self.server_version_cache.store(server_version, Ordering::Release);
    }

    fn handshake_context(&self) -> StartupHandshakeContext<'_> {
        StartupHandshakeContext {
            startup: self.startup_callback.as_deref(),
            notice_sink: &self.notice_broadcaster,
        }
    }

    /// Get a copy of the connection metadata
    pub async fn connection_metadata(&self) -> ConnectionMetadata {
        let mut metadata = self.connection_metadata.lock().await.clone();
        metadata.server_version = self.server_version_cache.load(Ordering::Acquire);
        metadata
    }

    /// Get the server version (lock-free; cached after handshake)
    pub(crate) fn server_version(&self) -> i32 {
        self.server_version_cache.load(Ordering::Acquire)
    }

    /// Reconnect to TWS with fibonacci backoff. Replays the handshake and
    /// re-fires the persisted startup / notice callbacks. When every attempt
    /// fails, returns the last attempt's error — a permanent cause (say, an
    /// incompatible server version) must not exit as a generic failure.
    ///
    /// Returns [`Error::Shutdown`] once shutdown is requested. The backoff
    /// wait ends immediately on the request; a connect already in flight is
    /// not interrupted, so the check runs again as soon as it returns.
    pub async fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);
        let mut last_error = None;

        let mut attempt: u32 = 0;
        while self.max_reconnect_attempts.is_none_or(|max| attempt < max) {
            if self.shutdown.is_requested() {
                return Err(Error::Shutdown);
            }

            attempt += 1;
            let attempt_label = match self.max_reconnect_attempts {
                Some(max) => format!("{attempt}/{max}"),
                None => attempt.to_string(),
            };
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            self.socket.sleep(next_delay, &self.shutdown).await;

            if self.shutdown.is_requested() {
                return Err(Error::Shutdown);
            }

            match self.socket.reconnect().await {
                Ok(_) => {
                    self.reset_connection_metadata().await;
                    match self.establish_connection().await {
                        Ok(()) => {
                            info!("reconnected");
                            return Ok(());
                        }
                        Err(e) => {
                            info!("reconnection attempt {attempt_label} failed while establishing session: {e}");
                            last_error = Some(e);
                        }
                    }
                }
                Err(e) => {
                    info!("reconnection attempt {attempt_label} failed: {e}");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(Error::ConnectionFailed))
    }

    async fn reset_connection_metadata(&self) {
        self.server_version_cache.store(0, Ordering::Release);

        let mut connection_metadata = self.connection_metadata.lock().await;
        *connection_metadata = ConnectionMetadata {
            client_id: self.client_id,
            ..Default::default()
        };
    }

    /// Establish connection to TWS
    pub(crate) async fn establish_connection(&self) -> Result<(), Error> {
        self.handshake().await?;
        require_protobuf_support(self.server_version())?;
        self.start_api().await?;
        self.receive_account_info().await?;
        Ok(())
    }

    /// Write a protobuf message to the connection
    pub(crate) async fn write_message(&self, data: &[u8]) -> Result<(), Error> {
        self.recorder.record_request(data);
        debug!("-> {:?}", data);

        self.write_raw(data).await
    }

    /// Read a message from the connection
    pub(crate) async fn read_message(&self) -> Response {
        let data = self.socket.read_message().await?;

        let message = parse_raw_message(&data)?;
        self.recorder.record_response(&message);

        Ok(message)
    }

    /// Write raw bytes with a length prefix
    pub(crate) async fn write_raw(&self, data: &[u8]) -> Result<(), Error> {
        let packet = encode_raw_length(data);
        self.socket.write_all(&packet).await?;
        Ok(())
    }

    // sends server handshake
    pub(crate) async fn handshake(&self) -> Result<(), Error> {
        let handshake = self.connection_handler.format_handshake();
        debug!("-> handshake: {handshake:?}");

        self.socket.write_all(&handshake).await?;

        let (server_version, time, tz) = parse_handshake_ack(&self.connection_handler, self.socket.read_message().await)?;

        let mut connection_metadata = self.connection_metadata.lock().await;
        self.server_version_cache.store(server_version, Ordering::Release);
        connection_metadata.connection_time = time;
        connection_metadata.time_zone = tz;
        Ok(())
    }

    // asks server to start processing messages
    pub(crate) async fn start_api(&self) -> Result<(), Error> {
        let server_version = self.server_version();
        let data = self.connection_handler.format_start_api(self.client_id, server_version);
        self.write_raw(&data).await?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) async fn receive_account_info(&self) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        let ctx = self.handshake_context();
        let server_version = self.server_version();
        loop {
            let mut message = self.read_message().await?;
            let info = self.connection_handler.parse_account_info(server_version, &mut message, &ctx)?;

            attempts += 1;
            if account_info.merge(info) || attempts > MAX_ACCOUNT_INFO_ATTEMPTS {
                break;
            }
        }

        self.connection_metadata.lock().await.apply_account_info(account_info);

        Ok(())
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
