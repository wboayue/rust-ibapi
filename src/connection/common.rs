//! Common connection logic shared between sync and async implementations

use std::fmt;
use std::sync::Arc;

use log::{debug, error, info, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::common::timezone::find_timezone;
use crate::errors::Error;
use crate::messages::{encode_length, encode_protobuf_message, IncomingMessages, OutgoingMessages, ResponseMessage, PROTOBUF_MSG_ID};
use crate::server_versions;

/// Callback for handling unsolicited messages during connection setup.
///
/// When TWS sends messages like `OpenOrder` or `OrderStatus` during the connection
/// handshake, this callback is invoked to allow the application to process them
/// instead of discarding them.
pub type StartupMessageCallback = Box<dyn Fn(ResponseMessage) + Send + Sync>;

/// Options for configuring a connection to TWS or IB Gateway.
///
/// Use the builder methods to configure options, then pass to
/// [`Client::connect_with_options`](crate::Client::connect_with_options).
///
/// # Examples
///
/// ```
/// use ibapi::ConnectionOptions;
///
/// let options = ConnectionOptions::default()
///     .tcp_no_delay(true);
/// ```
#[derive(Clone, Default)]
pub struct ConnectionOptions {
    pub(crate) tcp_no_delay: bool,
    pub(crate) startup_callback: Option<Arc<dyn Fn(ResponseMessage) + Send + Sync>>,
}

impl ConnectionOptions {
    /// Enable or disable `TCP_NODELAY` on the connection socket.
    ///
    /// When enabled, disables Nagle's algorithm for lower latency.
    /// Default: `false`.
    pub fn tcp_no_delay(mut self, enabled: bool) -> Self {
        self.tcp_no_delay = enabled;
        self
    }

    /// Set a callback for unsolicited messages during connection setup.
    ///
    /// When TWS sends messages like `OpenOrder` or `OrderStatus` during the
    /// connection handshake, this callback processes them instead of discarding.
    pub fn startup_callback(mut self, callback: impl Fn(ResponseMessage) + Send + Sync + 'static) -> Self {
        self.startup_callback = Some(Arc::new(callback));
        self
    }
}

impl From<Option<StartupMessageCallback>> for ConnectionOptions {
    fn from(callback: Option<StartupMessageCallback>) -> Self {
        let mut opts = Self::default();
        if let Some(cb) = callback {
            opts.startup_callback = Some(Arc::from(cb));
        }
        opts
    }
}

impl fmt::Debug for ConnectionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionOptions")
            .field("tcp_no_delay", &self.tcp_no_delay)
            .field("startup_callback", &self.startup_callback.is_some())
            .finish()
    }
}

/// Data exchanged during the connection handshake
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HandshakeData {
    pub min_version: i32,
    pub max_version: i32,
    pub server_version: i32,
    pub server_time: String,
}

/// Protocol for establishing connections to TWS
pub trait ConnectionProtocol {
    type Error;

    /// Format the initial handshake message
    fn format_handshake(&self) -> Vec<u8>;

    /// Parse the handshake response from the server
    fn parse_handshake_response(&self, response: &mut ResponseMessage) -> Result<HandshakeData, Self::Error>;

    /// Format the start API message as raw bytes (without length prefix).
    fn format_start_api(&self, client_id: i32, server_version: i32) -> Vec<u8>;

    /// Parse account information from incoming messages
    ///
    /// If a callback is provided, unsolicited messages (like OpenOrder, OrderStatus)
    /// will be passed to it instead of being discarded.
    fn parse_account_info(
        &self,
        message: &mut ResponseMessage,
        callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>,
    ) -> Result<AccountInfo, Self::Error>;
}

/// Account information received during connection establishment
#[derive(Debug, Clone, Default)]
pub struct AccountInfo {
    pub next_order_id: Option<i32>,
    pub managed_accounts: Option<String>,
}

/// Standard connection handler implementation
#[derive(Debug)]
pub struct ConnectionHandler {
    pub min_version: i32,
    pub max_version: i32,
}

impl Default for ConnectionHandler {
    fn default() -> Self {
        Self {
            min_version: server_versions::PROTOBUF,
            max_version: server_versions::UPDATE_CONFIG,
        }
    }
}

impl ConnectionProtocol for ConnectionHandler {
    type Error = Error;

    fn format_handshake(&self) -> Vec<u8> {
        let version_string = format!("v{}..{}", self.min_version, self.max_version);
        debug!("Handshake version: {version_string}");

        let mut handshake = Vec::from(b"API\0");
        handshake.extend_from_slice(&encode_length(&version_string));
        handshake
    }

    fn parse_handshake_response(&self, response: &mut ResponseMessage) -> Result<HandshakeData, Self::Error> {
        let server_version = response.next_int()?;
        let server_time = response.next_string()?;

        Ok(HandshakeData {
            min_version: self.min_version,
            max_version: self.max_version,
            server_version,
            server_time,
        })
    }

    fn format_start_api(&self, client_id: i32, _server_version: i32) -> Vec<u8> {
        use prost::Message;

        let request = crate::proto::StartApiRequest {
            client_id: Some(client_id),
            optional_capabilities: None,
        };

        encode_protobuf_message(OutgoingMessages::StartApi as i32, &request.encode_to_vec())
    }

    fn parse_account_info(
        &self,
        message: &mut ResponseMessage,
        callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>,
    ) -> Result<AccountInfo, Self::Error> {
        use prost::Message;

        let mut info = AccountInfo::default();

        match message.message_type() {
            IncomingMessages::NextValidId => {
                if message.is_protobuf {
                    if let Some(bytes) = message.raw_bytes() {
                        let proto =
                            crate::proto::NextValidId::decode(bytes).map_err(|e| Error::Simple(format!("failed to decode NextValidId: {e}")))?;
                        info.next_order_id = proto.order_id;
                    }
                } else {
                    message.skip(); // message type
                    message.skip(); // message version
                    info.next_order_id = Some(message.next_int()?);
                }
            }
            IncomingMessages::ManagedAccounts => {
                if message.is_protobuf {
                    if let Some(bytes) = message.raw_bytes() {
                        let proto = crate::proto::ManagedAccounts::decode(bytes)
                            .map_err(|e| Error::Simple(format!("failed to decode ManagedAccounts: {e}")))?;
                        info.managed_accounts = proto.accounts_list;
                    }
                } else {
                    message.skip(); // message type
                    message.skip(); // message version
                    info.managed_accounts = Some(message.next_string()?);
                }
            }
            IncomingMessages::Error => {
                let notice = if message.is_protobuf {
                    message.raw_bytes().and_then(|bytes| {
                        crate::proto::ErrorMessage::decode(bytes).ok().map(|proto| crate::messages::Notice {
                            code: proto.error_code.unwrap_or(0),
                            message: proto.error_msg.unwrap_or_default(),
                            error_time: None,
                            advanced_order_reject_json: proto.advanced_order_reject_json.unwrap_or_default(),
                        })
                    })
                } else {
                    Some(crate::messages::Notice::from(&*message))
                };
                if let Some(notice) = notice {
                    if notice.is_warning() || notice.is_system_message() {
                        info!("{notice}");
                    } else {
                        error!("Error during account info: {notice}");
                    }
                }
            }
            _ => {
                // Pass unsolicited messages to callback if provided
                if let Some(cb) = callback {
                    cb(message.clone());
                } else {
                    warn!(
                        "CONSUMING MESSAGE during connection setup: {:?} - THIS MESSAGE IS LOST!",
                        message.message_type()
                    );
                }
            }
        }

        Ok(info)
    }
}

/// Reject connections to TWS/IB Gateway builds older than the protobuf transport.
///
/// rust-ibapi 3.x is protobuf-only; `start_api` and every request encoder emit
/// protobuf, so a server below `server_versions::PROTOBUF` cannot interpret
/// what we send. Fail fast after the handshake with a descriptive error rather
/// than letting the gateway silently drop our messages.
pub(crate) fn require_protobuf_support(server_version: i32) -> Result<(), Error> {
    if server_version < server_versions::PROTOBUF {
        return Err(Error::ServerVersion(
            server_versions::PROTOBUF,
            server_version,
            format!(
                "protobuf transport — rust-ibapi 3.x requires TWS or IB Gateway with server version {} or later; please upgrade",
                server_versions::PROTOBUF
            ),
        ));
    }
    Ok(())
}

/// Parse connection time from TWS format
/// Format: "20230405 22:20:39 PST"
///
/// Returns `Err(Error::UnsupportedTimeZone)` when the gateway includes a timezone
/// name that is not in `TIMEZONE_ALIASES` and not a recognised IANA zone. Other
/// failure modes (truncated string, unparseable date) remain tolerant and yield
/// `Ok` with `None` for the affected component.
pub fn parse_connection_time(connection_time: &str) -> Result<(Option<OffsetDateTime>, Option<&'static Tz>), Error> {
    let parts: Vec<&str> = connection_time.split(' ').collect();

    if parts.len() < 3 {
        error!("Invalid connection time format: {connection_time}");
        return Ok((None, None));
    }

    // Combine timezone parts if more than 3 parts (e.g., "China Standard Time")
    let tz_name = if parts.len() > 3 { parts[2..].join(" ") } else { parts[2].to_string() };
    let zones = find_timezone(&tz_name);

    if zones.is_empty() {
        return Err(Error::UnsupportedTimeZone(tz_name));
    }

    let timezone = zones[0];

    let format = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let date_str = format!("{} {}", parts[0], parts[1]);
    let date = time::PrimitiveDateTime::parse(date_str.as_str(), format);

    match date {
        Ok(connected_at) => match connected_at.assume_timezone(timezone) {
            OffsetResult::Some(date) => Ok((Some(date), Some(timezone))),
            _ => {
                log::warn!("Error setting timezone");
                Ok((None, Some(timezone)))
            }
        },
        Err(err) => {
            log::warn!("Could not parse connection time from {date_str}: {err}");
            Ok((None, Some(timezone)))
        }
    }
}

/// Parse raw message bytes into a `ResponseMessage`, returning an optional debug string for tracing.
///
/// When `server_version >= PROTOBUF` and the 4-byte binary message ID exceeds 200,
/// the payload is protobuf-encoded. Otherwise the payload is NUL-delimited text.
pub fn parse_raw_message(data: &[u8], server_version: i32) -> (ResponseMessage, Option<String>) {
    if server_version >= server_versions::PROTOBUF && data.len() >= 4 {
        let msg_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        if msg_id > PROTOBUF_MSG_ID {
            let real_type = msg_id - PROTOBUF_MSG_ID;
            debug!("<- protobuf msg_id={real_type}");
            let message = ResponseMessage::from_protobuf(real_type, data[4..].to_vec(), server_version);
            (message, None)
        } else {
            // Binary message ID but text payload
            let raw_string = String::from_utf8_lossy(&data[4..]).into_owned();
            debug!("<- {raw_string:?}");
            let message = ResponseMessage::from_binary_text(msg_id, &raw_string, server_version);
            (message, Some(raw_string))
        }
    } else {
        let raw_string = String::from_utf8_lossy(data).into_owned();
        debug!("<- {raw_string:?}");
        let message = ResponseMessage::from(&raw_string).with_server_version(server_version);
        (message, Some(raw_string))
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
