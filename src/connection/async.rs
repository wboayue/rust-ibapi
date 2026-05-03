//! Asynchronous connection implementation

use std::sync::atomic::{AtomicI32, Ordering};

use log::{debug, info};

use super::common::{
    parse_connection_time, parse_raw_message, require_protobuf_support, AccountInfo, ConnectionHandler, ConnectionOptions, ConnectionProtocol,
    StartupMessageCallback,
};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_raw_length, ResponseMessage};
use crate::trace;
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::r#async::{AsyncStream, AsyncTcpSocket};
use crate::transport::recorder::MessageRecorder;
use tokio::sync::Mutex;

type Response = Result<ResponseMessage, Error>;

/// Asynchronous connection to TWS, generic over the underlying `AsyncStream`.
/// The default `AsyncTcpSocket` is the production wiring; tests can substitute
/// an in-memory stream to drive the bus deterministically.
#[derive(Debug)]
pub struct AsyncConnection<S: AsyncStream = AsyncTcpSocket> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) server_version_cache: AtomicI32,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
}

impl AsyncConnection<AsyncTcpSocket> {
    /// Create a new async connection
    #[allow(dead_code)]
    pub async fn connect(address: &str, client_id: i32) -> Result<Self, Error> {
        Self::connect_with_callback(address, client_id, None).await
    }

    /// Create a new async connection with a callback for unsolicited messages
    ///
    /// The callback will be invoked for any messages received during connection
    /// setup that are not part of the normal handshake (e.g., OpenOrder, OrderStatus).
    pub async fn connect_with_callback(address: &str, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Self, Error> {
        Self::connect_with_options(address, client_id, startup_callback.into()).await
    }

    /// Create a new async connection with custom options.
    ///
    /// Applies settings from [`ConnectionOptions`] (e.g. `TCP_NODELAY`, startup callback)
    /// before performing the TWS handshake.
    pub async fn connect_with_options(address: &str, client_id: i32, options: ConnectionOptions) -> Result<Self, Error> {
        let socket = AsyncTcpSocket::connect(address, options.tcp_no_delay).await?;
        let connection = Self::stubbed(socket, client_id);
        let cb_ref = options.startup_callback.as_deref();
        connection.establish_connection(cb_ref).await?;
        Ok(connection)
    }
}

impl<S: AsyncStream> AsyncConnection<S> {
    /// Build a connection over an arbitrary `AsyncStream` without performing
    /// the handshake. For tests; the production path uses `connect_*` which
    /// runs `establish_connection` immediately after construction.
    pub(crate) fn stubbed(socket: S, client_id: i32) -> Self {
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

    /// Reconnect to TWS with fibonacci backoff
    pub async fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);

        for i in 0..MAX_RECONNECT_ATTEMPTS {
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            self.socket.sleep(next_delay).await;

            match self.socket.reconnect().await {
                Ok(_) => {
                    info!("reconnected !!!");
                    // Reconnection doesn't use startup callback
                    self.establish_connection(None).await?;
                    return Ok(());
                }
                Err(e) => {
                    info!("reconnection attempt {}/{} failed: {e}", i + 1, MAX_RECONNECT_ATTEMPTS);
                }
            }
        }

        Err(Error::ConnectionFailed)
    }

    /// Establish connection to TWS
    pub(crate) async fn establish_connection(&self, startup_callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>) -> Result<(), Error> {
        self.handshake().await?;
        require_protobuf_support(self.server_version())?;
        self.start_api().await?;
        self.receive_account_info(startup_callback).await?;
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
                return Err(Error::Simple(format!("The server may be rejecting connections from this host: {err}")));
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
    pub(crate) async fn receive_account_info(&self, startup_callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message().await?;
            let info = self.connection_handler.parse_account_info(&mut message, startup_callback)?;

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
