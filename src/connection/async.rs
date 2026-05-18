//! Asynchronous connection implementation

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use log::{debug, info};
use tokio::sync::{broadcast, Mutex};

use super::common::{
    parse_connection_time, parse_raw_message, require_protobuf_support, AccountInfo, ConnectionHandler, ConnectionProtocol, StartupHandshakeContext,
    StartupMessage,
};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_raw_length, Notice, ResponseMessage};
use crate::trace;
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::r#async::{AsyncStream, AsyncTcpSocket};
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
    /// `self.connection.notice_sender`) and any pre-bound `NoticeStream` the
    /// user obtained from `ClientBuilder::connect_with_notice_stream`.
    pub(crate) notice_sender: broadcast::Sender<Notice>,
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
    ) -> Result<Self, Error> {
        let socket = AsyncTcpSocket::connect(address, tcp_no_delay).await?;
        let connection = Self::with_socket(socket, client_id, startup_callback, notice_sender);
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
            notice_sender,
        }
    }

    /// Build a connection over an arbitrary `AsyncStream` without performing
    /// the handshake. For tests; the production path uses `with_pieces` which
    /// runs `establish_connection` immediately after construction.
    #[cfg(test)]
    pub(crate) fn stubbed(socket: S, client_id: i32) -> Self {
        let (notice_sender, _) = broadcast::channel(crate::transport::r#async::BROADCAST_CHANNEL_CAPACITY);
        Self::with_socket(socket, client_id, None, notice_sender)
    }

    fn handshake_context(&self) -> StartupHandshakeContext<'_> {
        StartupHandshakeContext {
            startup: self.startup_callback.as_deref(),
            notice_sink: &self.notice_sender,
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
    /// re-fires the persisted startup / notice callbacks.
    pub async fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);

        for i in 0..MAX_RECONNECT_ATTEMPTS {
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            self.socket.sleep(next_delay).await;

            match self.socket.reconnect().await {
                Ok(_) => {
                    info!("reconnected !!!");
                    self.reset_connection_metadata().await;
                    self.establish_connection().await?;
                    return Ok(());
                }
                Err(e) => {
                    info!("reconnection attempt {}/{} failed: {e}", i + 1, MAX_RECONNECT_ATTEMPTS);
                }
            }
        }

        Err(Error::ConnectionFailed)
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

        let (message, trace_str) = parse_raw_message(&data, self.server_version());

        if let Some(raw_string) = trace_str {
            if log::log_enabled!(log::Level::Debug) {
                trace::record_response(raw_string).await;
            }
        }

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

        // Read handshake response as raw text, bypassing parse_raw_message
        // which would misinterpret it as binary when server_version >= PROTOBUF (on reconnect).
        let ack: Result<ResponseMessage, Error> = match self.socket.read_message().await {
            Ok(data) => {
                let raw_string = String::from_utf8_lossy(&data).into_owned();
                Ok(ResponseMessage::from(&raw_string))
            }
            Err(err) => Err(err),
        };

        let mut connection_metadata = self.connection_metadata.lock().await;

        match ack {
            Ok(mut response) => {
                let handshake_data = self.connection_handler.parse_handshake_response(&mut response)?;
                self.server_version_cache.store(handshake_data.server_version, Ordering::Release);

                let (time, tz) = parse_connection_time(&handshake_data.server_time)?;
                connection_metadata.connection_time = time;
                connection_metadata.time_zone = tz;
            }
            Err(Error::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::ConnectionRejected(format!(
                    "server may be rejecting connections from this host: {err}"
                )));
            }
            Err(err) => {
                return Err(err);
            }
        }
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
        const MAX_ATTEMPTS: i32 = 100;
        let ctx = self.handshake_context();
        let server_version = self.server_version();
        loop {
            let mut message = self.read_message().await?;
            let info = self.connection_handler.parse_account_info(server_version, &mut message, &ctx)?;

            // Merge received info
            if info.next_order_id.is_some() {
                account_info.next_order_id = info.next_order_id;
            }
            if info.managed_accounts.is_some() {
                account_info.managed_accounts = info.managed_accounts;
            }

            attempts += 1;
            if (account_info.next_order_id.is_some() && account_info.managed_accounts.is_some()) || attempts > MAX_ATTEMPTS {
                break;
            }
        }

        // Update connection metadata
        let mut connection_metadata = self.connection_metadata.lock().await;
        if let Some(next_order_id) = account_info.next_order_id {
            connection_metadata.next_order_id = next_order_id;
        }
        if let Some(managed_accounts) = account_info.managed_accounts {
            connection_metadata.managed_accounts = managed_accounts;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
