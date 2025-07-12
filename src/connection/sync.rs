//! Synchronous connection implementation

use std::sync::Mutex;

use log::{debug, info};

use super::common::{parse_connection_time, AccountInfo, ConnectionHandler, ConnectionProtocol};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::transport::recorder::MessageRecorder;
use crate::transport::sync::{FibonacciBackoff, Stream, MAX_RETRIES};

type Response = Result<ResponseMessage, Error>;

/// Synchronous connection to TWS
#[derive(Debug)]
pub struct Connection<S: Stream> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) max_retries: i32,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
}

impl<S: Stream> Connection<S> {
    /// Create a new connection
    pub fn connect(socket: S, client_id: i32) -> Result<Self, Error> {
        let connection = Self {
            client_id,
            socket,
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            max_retries: MAX_RETRIES,
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
        };

        connection.establish_connection()?;

        Ok(connection)
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

    /// Reconnect to TWS with fibonacci backoff
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
        self.start_api()?;
        self.receive_account_info()?;
        Ok(())
    }

    /// Write a message to the connection
    pub(crate) fn write_message(&self, message: &RequestMessage) -> Result<(), Error> {
        self.recorder.record_request(message);
        let encoded = message.encode();
        debug!("-> {encoded:?}");
        let length_encoded = crate::messages::encode_length(&encoded);
        self.socket.write_all(&length_encoded)?;
        Ok(())
    }

    /// Read a message from the connection
    pub(crate) fn read_message(&self) -> Response {
        let data = self.socket.read_message()?;
        let raw_string = String::from_utf8(data)?;
        debug!("<- {raw_string:?}");

        let message = ResponseMessage::from(&raw_string);

        self.recorder.record_response(&message);

        Ok(message)
    }

    // sends server handshake
    pub(crate) fn handshake(&self) -> Result<(), Error> {
        let handshake = self.connection_handler.format_handshake();
        debug!("-> handshake: {handshake:?}");

        self.socket.write_all(&handshake)?;

        let ack = self.read_message();

        let mut connection_metadata = self.connection_metadata.lock()?;

        match ack {
            Ok(mut response) => {
                let handshake_data = self.connection_handler.parse_handshake_response(&mut response)?;
                connection_metadata.server_version = handshake_data.server_version;

                let (time, tz) = parse_connection_time(&handshake_data.server_time);
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
        let message = self.connection_handler.format_start_api(self.client_id, server_version);
        self.write_message(&message)?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) fn receive_account_info(&self) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message()?;
            let info = self.connection_handler.parse_account_info(&mut message)?;

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
            max_retries: MAX_RETRIES,
            recorder: MessageRecorder::new(false, String::from("")),
            connection_handler: ConnectionHandler::default(),
        }
    }
}
