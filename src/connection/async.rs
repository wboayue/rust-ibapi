//! Asynchronous connection implementation

use log::{debug, error, info};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, Tz};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_length, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::server_versions;
use crate::transport::recorder::MessageRecorder;

type Response = Result<ResponseMessage, Error>;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE;

/// Asynchronous connection to TWS
#[derive(Debug)]
pub struct AsyncConnection {
    pub(crate) client_id: i32,
    pub(crate) socket: Mutex<TcpStream>,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) recorder: MessageRecorder,
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
        let length_encoded = encode_length(&encoded);

        let mut socket = self.socket.lock().await;
        socket.write_all(&length_encoded).await?;
        Ok(())
    }

    /// Read a message from the connection
    pub(crate) async fn read_message(&self) -> Response {
        // Read message length
        let mut length_bytes = [0u8; 4];
        {
            let mut socket = self.socket.lock().await;
            socket.read_exact(&mut length_bytes).await?;
        }

        let message_length = u32::from_be_bytes(length_bytes) as usize;

        // Read message data
        let mut data = vec![0u8; message_length];
        {
            let mut socket = self.socket.lock().await;
            socket.read_exact(&mut data).await?;
        }

        let raw_string = String::from_utf8(data)?;
        debug!("<- {:?}", raw_string);

        let message = ResponseMessage::from(&raw_string);

        self.recorder.record_response(&message);

        Ok(message)
    }

    // sends server handshake
    pub(crate) async fn handshake(&self) -> Result<(), Error> {
        let version = &format!("v{MIN_SERVER_VERSION}..{MAX_SERVER_VERSION}");

        debug!("-> {version:?}");

        let mut handshake = Vec::from(b"API\0");
        handshake.extend_from_slice(&encode_length(version));

        {
            let mut socket = self.socket.lock().await;
            socket.write_all(&handshake).await?;
        }

        let ack = self.read_message().await;

        let mut connection_metadata = self.connection_metadata.lock().await;

        match ack {
            Ok(mut response) => {
                connection_metadata.server_version = response.next_int()?;

                let time = response.next_string()?;
                (connection_metadata.connection_time, connection_metadata.time_zone) = parse_connection_time(time.as_str());
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
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&OutgoingMessages::StartApi);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version() > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.write_message(prelude).await?;

        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) async fn receive_account_info(&self) -> Result<(), Error> {
        let mut saw_next_order_id: bool = false;
        let mut saw_managed_accounts: bool = false;

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message().await?;

            match message.message_type() {
                IncomingMessages::NextValidId => {
                    saw_next_order_id = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock().await;
                    connection_metadata.next_order_id = message.next_int()?;
                }
                IncomingMessages::ManagedAccounts => {
                    saw_managed_accounts = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock().await;
                    connection_metadata.managed_accounts = message.next_string()?;
                }
                IncomingMessages::Error => {
                    error!("message: {message:?}")
                }
                _ => info!("message: {message:?}"),
            }

            attempts += 1;
            if (saw_next_order_id && saw_managed_accounts) || attempts > MAX_ATTEMPTS {
                break;
            }
        }

        Ok(())
    }
}

// Parses following format: 20230405 22:20:39 PST
fn parse_connection_time(connection_time: &str) -> (Option<OffsetDateTime>, Option<&'static Tz>) {
    let parts: Vec<&str> = connection_time.split(' ').collect();

    let zones = timezones::find_by_name(parts[2]);

    if zones.is_empty() {
        error!("time zone not found for {}", parts[2]);
        return (None, None);
    }

    let timezone = zones[0];

    let format = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let date_str = format!("{} {}", parts[0], parts[1]);
    let date = time::PrimitiveDateTime::parse(date_str.as_str(), format);
    match date {
        Ok(connected_at) => match connected_at.assume_timezone(timezone) {
            OffsetResult::Some(date) => (Some(date), Some(timezone)),
            _ => {
                log::warn!("error setting timezone");
                (None, Some(timezone))
            }
        },
        Err(err) => {
            log::warn!("could not parse connection time from {date_str}: {err}");
            (None, Some(timezone))
        }
    }
}
