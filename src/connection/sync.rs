//! Synchronous connection implementation

use std::sync::{Arc, Mutex};

use log::{debug, info};

use crate::messages::Notice;

use super::common::{
    parse_connection_time, parse_raw_message, require_protobuf_support, AccountInfo, ConnectionHandler, ConnectionOptions, ConnectionProtocol,
    StartupCallbacks, StartupMessage, StartupMessageCallback,
};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_raw_length, ResponseMessage};
use crate::trace;
use crate::transport::common::{FibonacciBackoff, MAX_RECONNECT_ATTEMPTS};
use crate::transport::recorder::MessageRecorder;
use crate::transport::sync::Stream;
use crate::transport::sync::TcpSocket;

type Response = Result<ResponseMessage, Error>;

/// Synchronous connection to TWS
pub struct Connection<S: Stream> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) max_retries: i32,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
    /// Persisted callbacks copied from `ConnectionOptions`. Both fire on the
    /// initial handshake *and* on every auto-reconnect.
    startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
    notice_callback: Option<Arc<dyn Fn(Notice) + Send + Sync>>,
}

impl<S: Stream> std::fmt::Debug for Connection<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("client_id", &self.client_id)
            .field("connection_metadata", &self.connection_metadata)
            .field("max_retries", &self.max_retries)
            .field("startup_callback", &self.startup_callback.is_some())
            .field("notice_callback", &self.notice_callback.is_some())
            .finish()
    }
}

impl Connection<TcpSocket> {
    /// Create a connection with custom options.
    ///
    /// Applies settings from [`ConnectionOptions`] (e.g. `TCP_NODELAY`, startup callbacks)
    /// before performing the TWS handshake. Callbacks persist across reconnects.
    pub fn connect_with_options(address: &str, client_id: i32, options: ConnectionOptions) -> Result<Self, Error> {
        let socket = TcpSocket::connect(address, options.tcp_no_delay)?;
        Self::init(socket, client_id, options.startup_callback, options.startup_notice_callback)
    }
}

impl<S: Stream> Connection<S> {
    /// Create a new connection
    #[allow(dead_code)]
    pub fn connect(socket: S, client_id: i32) -> Result<Self, Error> {
        Self::init(socket, client_id, None, None)
    }

    /// Create a new connection with a callback for unsolicited messages
    ///
    /// The callback fires for messages received during the handshake (initial
    /// connect *and* auto-reconnect) that are not part of `NextValidId` /
    /// `ManagedAccounts` — e.g. `OpenOrder`, `OrderStatus`, account updates.
    #[allow(dead_code)]
    pub fn connect_with_callback(socket: S, client_id: i32, startup_callback: Option<StartupMessageCallback>) -> Result<Self, Error> {
        let cb = startup_callback.map(|c| Arc::from(c) as Arc<dyn Fn(StartupMessage) + Send + Sync>);
        Self::init(socket, client_id, cb, None)
    }

    fn init(
        socket: S,
        client_id: i32,
        startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
        notice_callback: Option<Arc<dyn Fn(Notice) + Send + Sync>>,
    ) -> Result<Self, Error> {
        let connection = Self {
            client_id,
            socket,
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            max_retries: MAX_RECONNECT_ATTEMPTS,
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
            startup_callback,
            notice_callback,
        };

        connection.establish_connection()?;

        Ok(connection)
    }

    fn callbacks(&self) -> StartupCallbacks<'_> {
        StartupCallbacks {
            startup: self.startup_callback.as_deref(),
            notice: self.notice_callback.as_deref(),
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

    /// Reconnect to TWS with fibonacci backoff. Replays the handshake and
    /// re-fires the persisted startup / notice callbacks.
    pub fn reconnect(&self) -> Result<(), Error> {
        let mut backoff = FibonacciBackoff::new(30);

        for i in 0..self.max_retries {
            let next_delay = backoff.next_delay();
            info!("next reconnection attempt in {next_delay:#?}");

            self.socket.sleep(next_delay);

            match self.socket.reconnect() {
                Ok(_) => {
                    info!("reconnected !!!");
                    self.establish_connection()?;

                    return Ok(());
                }
                Err(e) => {
                    info!("reconnection attempt {}/{} failed: {e}", i + 1, self.max_retries);
                }
            }
        }

        Err(Error::ConnectionFailed)
    }

    /// Establish connection to TWS
    pub(crate) fn establish_connection(&self) -> Result<(), Error> {
        self.handshake()?;
        require_protobuf_support(self.server_version())?;
        self.start_api()?;
        self.receive_account_info()?;
        Ok(())
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
        let (message, trace_str) = parse_raw_message(&data, self.server_version());

        if let Some(raw_string) = trace_str {
            if log::log_enabled!(log::Level::Debug) {
                trace::blocking::record_response(raw_string);
            }
        }

        self.recorder.record_response(&message);

        Ok(message)
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

        // Read handshake response as raw text, bypassing parse_raw_message
        // which would misinterpret it as binary when server_version >= PROTOBUF (on reconnect).
        let ack: Result<ResponseMessage, Error> = match self.socket.read_message() {
            Ok(data) => {
                let raw_string = String::from_utf8_lossy(&data).into_owned();
                Ok(ResponseMessage::from(&raw_string))
            }
            Err(e) => Err(e),
        };

        let mut connection_metadata = self.connection_metadata.lock()?;

        match ack {
            Ok(mut response) => {
                let handshake_data = self.connection_handler.parse_handshake_response(&mut response)?;
                connection_metadata.server_version = handshake_data.server_version;

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
    pub(crate) fn start_api(&self) -> Result<(), Error> {
        let server_version = self.server_version();
        let data = self.connection_handler.format_start_api(self.client_id, server_version);
        self.write_raw(&data)?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) fn receive_account_info(&self) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        let callbacks = self.callbacks();
        let server_version = self.server_version();
        loop {
            let mut message = self.read_message()?;
            let info = self.connection_handler.parse_account_info(server_version, &mut message, &callbacks)?;

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
        let mut connection_metadata = self.connection_metadata.lock()?;
        if let Some(next_order_id) = account_info.next_order_id {
            connection_metadata.next_order_id = next_order_id;
        }
        if let Some(managed_accounts) = account_info.managed_accounts {
            connection_metadata.managed_accounts = managed_accounts;
        }

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
            max_retries: MAX_RECONNECT_ATTEMPTS,
            recorder: MessageRecorder::new(false, String::from("")),
            connection_handler: ConnectionHandler::default(),
            startup_callback: None,
            notice_callback: None,
        }
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
