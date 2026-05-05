//! Common connection logic shared between sync and async implementations

use std::fmt;
use std::sync::Arc;

use log::{debug, error, info, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::accounts::AccountUpdate;
use crate::common::timezone::find_timezone;
use crate::errors::Error;
use crate::messages::{encode_length, encode_protobuf_message, IncomingMessages, Notice, OutgoingMessages, ResponseMessage, PROTOBUF_MSG_ID};
use crate::orders::{OrderData, OrderStatus};
use crate::server_versions;

/// Domain-typed messages delivered to a [`StartupMessageCallback`] during the
/// connection handshake (initial connect *and* auto-reconnect).
///
/// TWS may emit any of these unsolicited at handshake time when the previous
/// session had outstanding orders, an active `reqAccountUpdates`, etc. Anything
/// not covered by a dedicated variant lands in `Other` so callers can still
/// inspect the raw message.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum StartupMessage {
    /// Open order — typed via the order decoders.
    OpenOrder(OrderData),
    /// Order status — typed via the order decoders.
    OrderStatus(OrderStatus),
    /// End-of-open-orders marker. TWS emits this after the last `OpenOrder`
    /// frame at handshake time so callers know they've seen the full set.
    /// No payload.
    OpenOrderEnd,
    /// Account update (`AccountValue`, `PortfolioValue`, `UpdateTime`, `End`).
    /// Reuses the existing [`AccountUpdate`] enum so the same patterns work at
    /// startup and at runtime.
    AccountUpdate(AccountUpdate),
    /// Anything else that arrived unsolicited during account-info exchange —
    /// e.g. `ExecutionData`, `CommissionReport`, `CompletedOrder`. Inspect via
    /// [`ResponseMessage::message_type`] and decode as needed.
    Other(ResponseMessage),
}

impl StartupMessage {
    /// The TWS message type that produced this startup message. Useful for
    /// telemetry / logging without unpacking the typed payload.
    pub fn message_type(&self) -> IncomingMessages {
        match self {
            StartupMessage::OpenOrder(_) => IncomingMessages::OpenOrder,
            StartupMessage::OrderStatus(_) => IncomingMessages::OrderStatus,
            StartupMessage::OpenOrderEnd => IncomingMessages::OpenOrderEnd,
            StartupMessage::AccountUpdate(au) => match au {
                AccountUpdate::AccountValue(_) => IncomingMessages::AccountValue,
                AccountUpdate::PortfolioValue(_) => IncomingMessages::PortfolioValue,
                AccountUpdate::UpdateTime(_) => IncomingMessages::AccountUpdateTime,
                AccountUpdate::End => IncomingMessages::AccountDownloadEnd,
            },
            StartupMessage::Other(rm) => rm.message_type(),
        }
    }
}

/// Callback for unsolicited typed messages emitted during the connection
/// handshake (initial connect *and* every auto-reconnect handshake).
pub type StartupMessageCallback = Box<dyn Fn(StartupMessage) + Send + Sync>;

/// Callback for IB notices emitted during the connection handshake — e.g. the
/// 2104/2106/2158 farm-status notices, 1100/1101/1102 connectivity codes.
/// Without this callback these are log-only.
///
/// Fires during initial connect *and* every auto-reconnect handshake.
pub type StartupNoticeCallback = Box<dyn Fn(Notice) + Send + Sync>;

/// Bundle of the two startup-time callbacks for internal threading.
///
/// Public API exposes them as separate builder methods on [`ConnectionOptions`];
/// internally they always travel together, so we pass one struct instead of two
/// callback parameters (project rule: ≤3 fn args).
pub(crate) struct StartupCallbacks<'a> {
    pub startup: Option<&'a (dyn Fn(StartupMessage) + Send + Sync)>,
    pub notice: Option<&'a (dyn Fn(Notice) + Send + Sync)>,
}

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
    pub(crate) startup_callback: Option<Arc<dyn Fn(StartupMessage) + Send + Sync>>,
    pub(crate) startup_notice_callback: Option<Arc<dyn Fn(Notice) + Send + Sync>>,
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

    /// Set a callback for unsolicited typed messages during connection setup.
    ///
    /// When TWS sends messages like `OpenOrder`, `OrderStatus`, or account-update
    /// frames during the connection handshake, this callback receives them as
    /// typed [`StartupMessage`] values instead of having them discarded.
    /// Fires during the initial connect *and* every auto-reconnect handshake.
    pub fn startup_callback(mut self, callback: impl Fn(StartupMessage) + Send + Sync + 'static) -> Self {
        self.startup_callback = Some(Arc::new(callback));
        self
    }

    /// Set a callback for notices emitted during connection setup.
    ///
    /// Receives the 2104/2106/2158 farm-status notices, 1100/1101/1102
    /// connectivity codes, and any other handshake-time error/warning messages
    /// as typed [`Notice`] values. Without this callback these messages are
    /// log-only. Fires during the initial connect *and* every auto-reconnect
    /// handshake.
    pub fn startup_notice_callback(mut self, callback: impl Fn(Notice) + Send + Sync + 'static) -> Self {
        self.startup_notice_callback = Some(Arc::new(callback));
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
            .field("startup_notice_callback", &self.startup_notice_callback.is_some())
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

    /// Parse account information from incoming messages.
    ///
    /// `NextValidId` and `ManagedAccounts` are consumed internally to populate
    /// [`AccountInfo`]. Anything else is delegated to
    /// [`dispatch_unsolicited_message`] so the callbacks decide how to surface
    /// (or drop) it.
    fn parse_account_info(
        &self,
        server_version: i32,
        message: &mut ResponseMessage,
        callbacks: &StartupCallbacks<'_>,
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
        server_version: i32,
        message: &mut ResponseMessage,
        callbacks: &StartupCallbacks<'_>,
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
            _ => dispatch_unsolicited_message(server_version, message, callbacks),
        }

        Ok(info)
    }
}

/// Dispatch an unsolicited message that arrived during the handshake — i.e. one
/// that wasn't `NextValidId` / `ManagedAccounts` (which `parse_account_info`
/// consumes itself). Errors fan out to the notice callback; `OpenOrder` /
/// `OrderStatus` / account-update frames are decoded into typed
/// [`StartupMessage`] values; anything else lands in `StartupMessage::Other` so
/// the caller can still inspect it. Decode failures surface as `Other` rather
/// than being silently dropped.
pub(crate) fn dispatch_unsolicited_message(server_version: i32, message: &mut ResponseMessage, callbacks: &StartupCallbacks<'_>) {
    use crate::accounts::common::decode_account_update_either;
    use crate::orders::common::{decode_open_order_either, decode_order_status_either};
    use crate::transport::routing::decode_error_envelope;

    match message.message_type() {
        IncomingMessages::Error => {
            // Reuse the dispatcher's protobuf Error decoder + DecodedError→Notice
            // conversion so handshake notices preserve `error_time` (millis →
            // OffsetDateTime) the same way runtime notices do.
            let notice = if message.is_protobuf {
                message.raw_bytes().and_then(decode_error_envelope).map(Notice::from)
            } else {
                Some(Notice::from(&*message))
            };
            if let Some(notice) = notice {
                if notice.is_warning() || notice.is_system_message() {
                    info!("{notice}");
                } else {
                    error!("Error during account info: {notice}");
                }
                if let Some(cb) = callbacks.notice {
                    cb(notice);
                }
            }
        }
        IncomingMessages::OpenOrder => {
            if let Some(cb) = callbacks.startup {
                let typed = decode_open_order_either(server_version, message)
                    .map(StartupMessage::OpenOrder)
                    .unwrap_or_else(|_| StartupMessage::Other(message.clone()));
                cb(typed);
            }
        }
        IncomingMessages::OrderStatus => {
            if let Some(cb) = callbacks.startup {
                let typed = decode_order_status_either(server_version, message)
                    .map(StartupMessage::OrderStatus)
                    .unwrap_or_else(|_| StartupMessage::Other(message.clone()));
                cb(typed);
            }
        }
        IncomingMessages::OpenOrderEnd => {
            if let Some(cb) = callbacks.startup {
                cb(StartupMessage::OpenOrderEnd);
            }
        }
        IncomingMessages::AccountValue
        | IncomingMessages::PortfolioValue
        | IncomingMessages::AccountUpdateTime
        | IncomingMessages::AccountDownloadEnd => {
            if let Some(cb) = callbacks.startup {
                let typed = decode_account_update_either(server_version, message)
                    .map(StartupMessage::AccountUpdate)
                    .unwrap_or_else(|_| StartupMessage::Other(message.clone()));
                cb(typed);
            }
        }
        _ => {
            if let Some(cb) = callbacks.startup {
                cb(StartupMessage::Other(message.clone()));
            } else {
                warn!(
                    "CONSUMING MESSAGE during connection setup: {:?} - THIS MESSAGE IS LOST!",
                    message.message_type()
                );
            }
        }
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
