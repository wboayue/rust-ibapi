//! Synchronous connection implementation

use std::sync::Mutex;

use log::{debug, error, info};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, Tz};

use super::ConnectionMetadata;
use crate::errors::Error;
use crate::messages::{encode_length, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::server_versions;
use crate::transport::recorder::MessageRecorder;
use crate::transport::sync::{FibonacciBackoff, Stream, MAX_RETRIES};

type Response = Result<ResponseMessage, Error>;

const MIN_SERVER_VERSION: i32 = 100;
const MAX_SERVER_VERSION: i32 = crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE;

/// Synchronous connection to TWS
#[derive(Debug)]
pub struct Connection<S: Stream> {
    pub(crate) client_id: i32,
    pub(crate) socket: S,
    pub(crate) connection_metadata: Mutex<ConnectionMetadata>,
    pub(crate) max_retries: i32,
    pub(crate) recorder: MessageRecorder,
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
        let length_encoded = encode_length(&encoded);
        self.socket.write_all(&length_encoded)?;
        Ok(())
    }

    /// Read a message from the connection
    pub(crate) fn read_message(&self) -> Response {
        let data = self.socket.read_message()?;
        let raw_string = String::from_utf8(data)?;
        debug!("<- {:?}", raw_string);

        let message = ResponseMessage::from(&raw_string);

        self.recorder.record_response(&message);

        Ok(message)
    }

    // sends server handshake
    pub(crate) fn handshake(&self) -> Result<(), Error> {
        let version = &format!("v{MIN_SERVER_VERSION}..{MAX_SERVER_VERSION}");

        debug!("-> {version:?}");

        let mut handshake = Vec::from(b"API\0");
        handshake.extend_from_slice(&encode_length(version));

        self.socket.write_all(&handshake)?;

        let ack = self.read_message();

        let mut connection_metadata = self.connection_metadata.lock()?;

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
    pub(crate) fn start_api(&self) -> Result<(), Error> {
        const VERSION: i32 = 2;

        let prelude = &mut RequestMessage::default();

        prelude.push_field(&OutgoingMessages::StartApi);
        prelude.push_field(&VERSION);
        prelude.push_field(&self.client_id);

        if self.server_version() > server_versions::OPTIONAL_CAPABILITIES {
            prelude.push_field(&"");
        }

        self.write_message(prelude)?;

        Ok(())
    }

    // Fetches next order id and managed accounts.
    pub(crate) fn receive_account_info(&self) -> Result<(), Error> {
        let mut saw_next_order_id: bool = false;
        let mut saw_managed_accounts: bool = false;

        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 100;
        loop {
            let mut message = self.read_message()?;

            match message.message_type() {
                IncomingMessages::NextValidId => {
                    saw_next_order_id = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock()?;
                    connection_metadata.next_order_id = message.next_int()?;
                }
                IncomingMessages::ManagedAccounts => {
                    saw_managed_accounts = true;

                    message.skip(); // message type
                    message.skip(); // message version

                    let mut connection_metadata = self.connection_metadata.lock()?;
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;
    use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

    #[test]
    fn test_parse_connection_time() {
        let example = "20230405 22:20:39 PST";
        let (connection_time, _) = parse_connection_time(example);

        let la = timezones::db::america::LOS_ANGELES;
        if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
            assert_eq!(connection_time, Some(other));
        }
    }
}
