//! Synchronous connection implementation

use std::sync::{Arc, Mutex};

use log::{debug, info};

use super::common::{
    parse_handshake_ack, parse_raw_message, require_protobuf_support, AccountInfo, ConnectionConfig, ConnectionHandler, ConnectionProtocol,
    StartupCallback, StartupHandshakeContext, MAX_ACCOUNT_INFO_ATTEMPTS,
};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_raw_length, ResponseMessage};
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::recorder::MessageRecorder;
use crate::transport::sync::{NoticeBroadcaster, ShutdownSignal, Stream, TcpSocket, STARTUP_TIMEOUT_LIMIT};

type Response = Result<ResponseMessage, Error>;

/// Synchronous connection to TWS
pub struct Connection<S: Stream> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    /// Reconnection attempts before `reconnect` gives up; `None` retries
    /// forever. Defaults to `Some(`[`MAX_RECONNECT_ATTEMPTS`]`)`.
    pub(crate) max_reconnect_attempts: Option<u32>,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
    /// Optional typed-message callback supplied via [`ClientBuilder::startup_callback`].
    /// Fires on initial handshake *and* every auto-reconnect handshake.
    startup_callback: Option<StartupCallback>,
    /// Fan-out for unrouted notices. Shared with the bus (the bus reads via
    /// `self.connection.notice_broadcaster`) and any pre-bound `NoticeStream`
    /// the user obtained from `ClientBuilder::connect_with_notice_stream`.
    pub(crate) notice_broadcaster: Arc<NoticeBroadcaster>,
    /// Shared with the bus so `reconnect` can abandon its backoff as soon as
    /// shutdown is requested. See [`Connection::shutdown_signal`].
    shutdown: Arc<ShutdownSignal>,
    /// The last `RequestMarketDataType` packet requested. TWS does not keep
    /// the market data type across reconnects, so `establish_connection`
    /// re-sends it after every reconnect handshake.
    market_data_type_request: Mutex<Option<Vec<u8>>>,
}

impl<S: Stream> std::fmt::Debug for Connection<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("client_id", &self.client_id)
            .field("connection_metadata", &self.connection_metadata)
            .field("max_reconnect_attempts", &self.max_reconnect_attempts)
            .field("startup_callback", &self.startup_callback.is_some())
            .finish()
    }
}

impl Connection<TcpSocket> {
    /// Open the socket and run the TWS handshake. Settings come from the
    /// builder's validated `ConnectionConfig`.
    ///
    /// `notice_broadcaster` is shared with any pre-bound `NoticeStream`; the
    /// builder allocates it (and the matching receiver) before calling here.
    /// Persists across reconnects.
    pub(crate) fn connect(config: ConnectionConfig, notice_broadcaster: Arc<NoticeBroadcaster>) -> Result<Self, Error> {
        let socket = TcpSocket::connect(&config.address, config.tcp_no_delay)?;
        let mut connection = Self::with_socket(socket, config.client_id, config.startup_callback, notice_broadcaster);
        connection.max_reconnect_attempts = config.max_reconnect_attempts;
        connection.establish_connection()?;
        Ok(connection)
    }
}

impl<S: Stream> Connection<S> {
    /// Build a `Connection<S>` over an arbitrary `Stream`. **Does not** run the
    /// handshake — caller is responsible for invoking `establish_connection`.
    /// Mirrors `AsyncConnection::with_socket`.
    pub(crate) fn with_socket(
        socket: S,
        client_id: i32,
        startup_callback: Option<StartupCallback>,
        notice_broadcaster: Arc<NoticeBroadcaster>,
    ) -> Self {
        Self {
            client_id,
            socket,
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            max_reconnect_attempts: Some(MAX_RECONNECT_ATTEMPTS),
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
            startup_callback,
            notice_broadcaster,
            shutdown: Arc::new(ShutdownSignal::default()),
            market_data_type_request: Mutex::new(None),
        }
    }

    /// Handle on the shutdown flag `reconnect` observes. The bus takes a
    /// clone and requests shutdown through it.
    pub(crate) fn shutdown_signal(&self) -> Arc<ShutdownSignal> {
        Arc::clone(&self.shutdown)
    }

    fn handshake_context(&self) -> StartupHandshakeContext<'_> {
        StartupHandshakeContext {
            client_id: self.client_id,
            startup: self.startup_callback.as_deref(),
            notice_sink: &*self.notice_broadcaster,
        }
    }

    /// Get a copy of the connection metadata
    pub fn connection_metadata(&self) -> ConnectionMetadata {
        let metadata = self.connection_metadata.lock().unwrap();
        metadata.clone()
    }

    /// Get the server version
    pub(crate) fn server_version(&self) -> i32 {
        let connection_metadata = self.connection_metadata.lock().unwrap();
        connection_metadata.server_version
    }

    /// Reconnect to TWS with fibonacci backoff. Replays the handshake,
    /// re-fires the persisted startup / notice callbacks and re-sends the
    /// remembered market data type. When every attempt fails, returns the
    /// last attempt's error — a permanent cause (say, an incompatible server
    /// version) must not exit as a generic failure.
    ///
    /// Every failure is retried, [`Error::ClientIdInUse`] included: TWS can
    /// hold the dropped session's client id until it notices that socket is
    /// dead (live: 326 on each attempt until then, then a clean reconnect).
    ///
    /// Returns [`Error::Shutdown`] once shutdown is requested. The backoff
    /// wait ends immediately on the request; a connect already in flight is
    /// not interrupted, so the check runs again as soon as it returns.
    pub fn reconnect(&self) -> Result<(), Error> {
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

            self.socket.sleep(next_delay, &self.shutdown);

            if self.shutdown.is_requested() {
                return Err(Error::Shutdown);
            }

            match self.socket.reconnect() {
                Ok(_) => {
                    self.reset_connection_metadata();
                    match self.establish_connection() {
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

    fn reset_connection_metadata(&self) {
        let mut connection_metadata = self.connection_metadata.lock().unwrap();
        *connection_metadata = ConnectionMetadata {
            client_id: self.client_id,
            ..Default::default()
        };
    }

    /// Establish connection to TWS
    pub(crate) fn establish_connection(&self) -> Result<(), Error> {
        self.handshake()?;
        require_protobuf_support(self.server_version())?;
        self.start_api()?;
        self.receive_account_info()?;
        self.replay_market_data_type()?;
        Ok(())
    }

    /// Keep `packet`, a `RequestMarketDataType`, for replay on reconnect.
    pub(crate) fn remember_market_data_type(&self, packet: Vec<u8>) {
        *self.market_data_type_request.lock().unwrap() = Some(packet);
    }

    /// Re-send the remembered market data type to the new session. No-op
    /// until `remember_market_data_type` has been called.
    fn replay_market_data_type(&self) -> Result<(), Error> {
        let packet = self.market_data_type_request.lock().unwrap().clone();
        match packet {
            Some(packet) => self.write_message(&packet),
            None => Ok(()),
        }
    }

    /// Write a protobuf message to the connection
    pub(crate) fn write_message(&self, data: &[u8]) -> Result<(), Error> {
        self.recorder.record_request(data);
        debug!("-> {:?}", data);

        self.write_raw(data)
    }

    /// Read a message from the connection
    pub(crate) fn read_message(&self) -> Response {
        let data = self.socket.read_message()?;
        self.decode_frame(&data)
    }

    fn decode_frame(&self, data: &[u8]) -> Response {
        let message = parse_raw_message(data)?;
        self.recorder.record_response(&message);

        Ok(message)
    }

    /// Read one startup frame body, waiting out read timeouts: a loaded
    /// gateway can take longer than one socket timeout to answer. Gives up
    /// after [`STARTUP_TIMEOUT_LIMIT`] consecutive timeouts.
    fn read_startup_frame(&self) -> Result<Vec<u8>, Error> {
        let mut timeouts = 0;
        loop {
            match self.socket.read_message() {
                Err(e) if e.is_read_timeout() => {
                    timeouts += 1;
                    if timeouts >= STARTUP_TIMEOUT_LIMIT {
                        return Err(Error::Io(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("no startup reply from TWS after {timeouts} read timeouts"),
                        )));
                    }
                }
                result => return result,
            }
        }
    }

    /// Write raw bytes with a length prefix
    pub(crate) fn write_raw(&self, data: &[u8]) -> Result<(), Error> {
        let packet = encode_raw_length(data);
        self.socket.write_all(&packet)?;
        Ok(())
    }

    // sends server handshake
    pub(crate) fn handshake(&self) -> Result<(), Error> {
        let handshake = self.connection_handler.format_handshake();
        debug!("-> handshake: {handshake:?}");

        self.socket.write_all(&handshake)?;

        let ack = parse_handshake_ack(&self.connection_handler, self.read_startup_frame())?;

        let mut connection_metadata = self.connection_metadata.lock()?;
        connection_metadata.server_version = ack.server_version;
        connection_metadata.connection_time = ack.connection_time;
        connection_metadata.time_zone = ack.time_zone;
        Ok(())
    }

    // asks server to start processing messages
    pub(crate) fn start_api(&self) -> Result<(), Error> {
        let data = self.connection_handler.format_start_api(self.client_id);
        self.write_raw(&data)?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) fn receive_account_info(&self) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        let ctx = self.handshake_context();
        loop {
            let message = self.decode_frame(&self.read_startup_frame()?)?;
            let info = self.connection_handler.parse_account_info(&message, &ctx)?;

            attempts += 1;
            if account_info.merge(info) || attempts > MAX_ACCOUNT_INFO_ATTEMPTS {
                break;
            }
        }

        self.connection_metadata.lock()?.apply_account_info(account_info);

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn stubbed(socket: S, client_id: i32) -> Connection<S> {
        Connection {
            client_id,
            socket,
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            max_reconnect_attempts: Some(MAX_RECONNECT_ATTEMPTS),
            recorder: MessageRecorder::disabled(),
            connection_handler: ConnectionHandler::default(),
            startup_callback: None,
            notice_broadcaster: Arc::new(NoticeBroadcaster::new()),
            shutdown: Arc::new(ShutdownSignal::default()),
            market_data_type_request: Mutex::new(None),
        }
    }

    /// Pin a post-handshake server version on a stubbed connection so
    /// `parse_raw_message` sees frames in the binary-text-payload / proto
    /// regime without going through `establish_connection`.
    #[cfg(test)]
    pub(crate) fn set_server_version_for_test(&self, server_version: i32) {
        self.connection_metadata.lock().unwrap().server_version = server_version;
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
