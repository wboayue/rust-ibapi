//! Asynchronous connection implementation

use std::sync::Arc;

use log::debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::common::{parse_connection_time, AccountInfo, ConnectionHandler, ConnectionProtocol};
use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{RequestMessage, ResponseMessage};
use crate::trace;
use crate::transport::recorder::MessageRecorder;

type Response = Result<ResponseMessage, Error>;

/// Asynchronous connection to TWS
#[derive(Debug)]
pub struct AsyncConnection {
    pub(crate) client_id: i32,
    pub(crate) socket: Mutex<TcpStream>,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) recorder: MessageRecorder,
    pub(crate) connection_handler: ConnectionHandler,
}

impl AsyncConnection {
    /// Create a new async connection
    pub async fn connect(address: &str, client_id: i32) -> Result<Self, Error> {
        let socket = TcpStream::connect(address).await?;

        let connection = Self {
            client_id,
            socket: Mutex::new(socket),
            connection_metadata: Mutex::new(ConnectionMetadata {
                client_id,
                ..Default::default()
            }),
            recorder: MessageRecorder::from_env(),
            connection_handler: ConnectionHandler::default(),
        };

        connection.establish_connection().await?;

        Ok(connection)
    }

    /// Get a copy of the connection metadata
    pub fn connection_metadata(&self) -> ConnectionMetadata {
        // For now, we'll use blocking lock since this is called during initialization
        // In a more complete implementation, this would be async
        futures::executor::block_on(async {
            let metadata = self.connection_metadata.lock().await;
            metadata.clone()
        })
    }

    /// Get the server version
    pub(crate) fn server_version(&self) -> i32 {
        // For now, we'll use blocking lock since this is called during initialization
        // In a more complete implementation, this would be async
        futures::executor::block_on(async {
            let connection_metadata = self.connection_metadata.lock().await;
            connection_metadata.server_version
        })
    }

    /// Establish connection to TWS
    pub(crate) async fn establish_connection(&self) -> Result<(), Error> {
        self.handshake().await?;
        self.start_api().await?;
        self.receive_account_info().await?;
        Ok(())
    }

    /// Write a message to the connection
    pub(crate) async fn write_message(&self, message: &RequestMessage) -> Result<(), Error> {
        self.recorder.record_request(message);
        let encoded = message.encode();
        debug!("-> {encoded:?}");

        // Record the request if debug logging is enabled
        if log::log_enabled!(log::Level::Debug) {
            trace::record_request(encoded.clone()).await;
        }

        let length_encoded = crate::messages::encode_length(&encoded);

        let mut socket = self.socket.lock().await;
        socket.write_all(&length_encoded).await?;
        socket.flush().await?;
        Ok(())
    }

    /// Read a message from the connection
    pub(crate) async fn read_message(&self) -> Response {
        // Read message length
        let mut length_bytes = [0u8; 4];
        {
            let mut socket = self.socket.lock().await;
            match socket.read_exact(&mut length_bytes).await {
                Ok(_) => {}
                Err(e) => {
                    debug!("Error reading message length: {:?}", e);
                    return Err(Error::Io(Arc::new(e)));
                }
            }
        }

        let message_length = u32::from_be_bytes(length_bytes) as usize;

        // Read message data
        let mut data = vec![0u8; message_length];
        {
            let mut socket = self.socket.lock().await;
            socket.read_exact(&mut data).await?;
        }

        let raw_string = String::from_utf8(data)?;
        debug!("<- {raw_string:?}");

        // Record the response if debug logging is enabled
        if log::log_enabled!(log::Level::Debug) {
            trace::record_response(raw_string.clone()).await;
        }

        let message = ResponseMessage::from(&raw_string);

        self.recorder.record_response(&message);

        Ok(message)
    }

    // sends server handshake
    pub(crate) async fn handshake(&self) -> Result<(), Error> {
        let handshake = self.connection_handler.format_handshake();
        debug!("-> handshake: {handshake:?}");

        {
            let mut socket = self.socket.lock().await;
            socket.write_all(&handshake).await?;
        }

        let ack = self.read_message().await;

        let mut connection_metadata = self.connection_metadata.lock().await;

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
    pub(crate) async fn start_api(&self) -> Result<(), Error> {
        let server_version = self.server_version();
        let message = self.connection_handler.format_start_api(self.client_id, server_version);
        self.write_message(&message).await?;
        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) async fn receive_account_info(&self) -> Result<(), Error> {
        let mut account_info = AccountInfo::default();

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message().await?;
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
