//! Common connection logic shared between sync and async implementations

use std::fmt;
use std::sync::Arc;

use log::{debug, error, info, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::common::timezone::find_timezone;
use crate::errors::Error;
use crate::messages::{encode_length, encode_protobuf_message, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage, PROTOBUF_MSG_ID};
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

    fn format_start_api(&self, client_id: i32, server_version: i32) -> Vec<u8> {
        if server_version >= server_versions::PROTOBUF {
            use prost::Message;

            let request = crate::proto::StartApiRequest {
                client_id: Some(client_id),
                optional_capabilities: None,
            };

            encode_protobuf_message(OutgoingMessages::StartApi as i32, &request.encode_to_vec())
        } else {
            // Legacy text format for older servers
            let mut message = RequestMessage::default();
            message.push_field(&OutgoingMessages::StartApi);
            message.push_field(&2i32); // VERSION
            message.push_field(&client_id);
            if server_version > server_versions::OPTIONAL_CAPABILITIES {
                message.push_field(&"");
            }
            message.encode().as_bytes().to_vec()
        }
    }

    fn parse_account_info(
        &self,
        message: &mut ResponseMessage,
        callback: Option<&(dyn Fn(ResponseMessage) + Send + Sync)>,
    ) -> Result<AccountInfo, Self::Error> {
        let mut info = AccountInfo::default();

        match message.message_type() {
            IncomingMessages::NextValidId => {
                message.skip(); // message type
                message.skip(); // message version
                info.next_order_id = Some(message.next_int()?);
            }
            IncomingMessages::ManagedAccounts => {
                message.skip(); // message type
                message.skip(); // message version
                info.managed_accounts = Some(message.next_string()?);
            }
            IncomingMessages::Error => {
                let notice = crate::messages::Notice::from(message);
                if notice.is_warning() || notice.is_system_message() {
                    info!("{notice}");
                } else {
                    error!("Error during account info: {notice}");
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

/// Parse connection time from TWS format
/// Format: "20230405 22:20:39 PST"
pub fn parse_connection_time(connection_time: &str) -> (Option<OffsetDateTime>, Option<&'static Tz>) {
    let parts: Vec<&str> = connection_time.split(' ').collect();

    if parts.len() < 3 {
        error!("Invalid connection time format: {connection_time}");
        return (None, None);
    }

    // Combine timezone parts if more than 3 parts (e.g., "China Standard Time")
    let tz_name = if parts.len() > 3 { parts[2..].join(" ") } else { parts[2].to_string() };
    let zones = find_timezone(&tz_name);

    if zones.is_empty() {
        error!("Time zone not found for {}", tz_name);
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
                log::warn!("Error setting timezone");
                (None, Some(timezone))
            }
        },
        Err(err) => {
            log::warn!("Could not parse connection time from {date_str}: {err}");
            (None, Some(timezone))
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
            let mut fields = vec![msg_id.to_string()];
            fields.extend(raw_string.split_terminator('\0').map(|s| s.to_string()));
            let message = ResponseMessage {
                i: 0,
                fields,
                server_version,
                is_protobuf: false,
                raw_bytes: None,
            };
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
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use time::macros::datetime;
    use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, TimeZone};

    #[test]
    fn test_parse_account_info_next_valid_id() {
        let handler = ConnectionHandler::default();
        // NextValidId message: message_type=9, version=1, next_order_id=1000
        let mut message = ResponseMessage::from("9\01\01000\0");

        let result = handler.parse_account_info(&mut message, None);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.next_order_id, Some(1000));
        assert_eq!(info.managed_accounts, None);
    }

    #[test]
    fn test_parse_account_info_managed_accounts() {
        let handler = ConnectionHandler::default();
        // ManagedAccounts message: message_type=15, version=1, accounts="DU123,DU456"
        let mut message = ResponseMessage::from("15\01\0DU123,DU456\0");

        let result = handler.parse_account_info(&mut message, None);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.next_order_id, None);
        assert_eq!(info.managed_accounts, Some("DU123,DU456".to_string()));
    }

    #[test]
    fn test_parse_account_info_callback_invoked_for_open_order() {
        let handler = ConnectionHandler::default();
        // OpenOrder message: message_type=5
        let mut message = ResponseMessage::from("5\0123\0AAPL\0STK\0");

        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        let callback: StartupMessageCallback = Box::new(move |_msg| {
            *callback_invoked_clone.lock().unwrap() = true;
        });

        let result = handler.parse_account_info(&mut message, Some(&callback));
        assert!(result.is_ok());

        assert!(*callback_invoked.lock().unwrap(), "callback should be invoked for OpenOrder");
    }

    #[test]
    fn test_parse_account_info_callback_invoked_for_order_status() {
        let handler = ConnectionHandler::default();
        // OrderStatus message: message_type=3
        let mut message = ResponseMessage::from("3\0456\0Filled\0100\0");

        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        let callback: StartupMessageCallback = Box::new(move |_msg| {
            *callback_invoked_clone.lock().unwrap() = true;
        });

        let result = handler.parse_account_info(&mut message, Some(&callback));
        assert!(result.is_ok());

        assert!(*callback_invoked.lock().unwrap(), "callback should be invoked for OrderStatus");
    }

    #[test]
    fn test_parse_account_info_callback_receives_message() {
        let handler = ConnectionHandler::default();
        // OpenOrder message with identifiable content
        let mut message = ResponseMessage::from("5\0999\0TEST_SYMBOL\0");

        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let received_messages_clone = received_messages.clone();

        let callback: StartupMessageCallback = Box::new(move |msg| {
            received_messages_clone.lock().unwrap().push(msg);
        });

        let result = handler.parse_account_info(&mut message, Some(&callback));
        assert!(result.is_ok());

        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type(), IncomingMessages::OpenOrder);
    }

    #[test]
    fn test_parse_account_info_callback_not_invoked_for_next_valid_id() {
        let handler = ConnectionHandler::default();
        // NextValidId message should NOT trigger callback
        let mut message = ResponseMessage::from("9\01\01000\0");

        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        let callback: StartupMessageCallback = Box::new(move |_msg| {
            *callback_invoked_clone.lock().unwrap() = true;
        });

        let result = handler.parse_account_info(&mut message, Some(&callback));
        assert!(result.is_ok());

        assert!(!*callback_invoked.lock().unwrap(), "callback should NOT be invoked for NextValidId");
    }

    #[test]
    fn test_parse_account_info_callback_not_invoked_for_managed_accounts() {
        let handler = ConnectionHandler::default();
        // ManagedAccounts message should NOT trigger callback
        let mut message = ResponseMessage::from("15\01\0DU123\0");

        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        let callback: StartupMessageCallback = Box::new(move |_msg| {
            *callback_invoked_clone.lock().unwrap() = true;
        });

        let result = handler.parse_account_info(&mut message, Some(&callback));
        assert!(result.is_ok());

        assert!(!*callback_invoked.lock().unwrap(), "callback should NOT be invoked for ManagedAccounts");
    }

    #[test]
    fn test_parse_account_info_multiple_messages_callback() {
        let handler = ConnectionHandler::default();
        let received_count = Arc::new(Mutex::new(0));
        let received_count_clone = received_count.clone();

        let callback: StartupMessageCallback = Box::new(move |_msg| {
            *received_count_clone.lock().unwrap() += 1;
        });

        // First message: OpenOrder
        let mut msg1 = ResponseMessage::from("5\0123\0AAPL\0");
        handler.parse_account_info(&mut msg1, Some(&callback)).unwrap();

        // Second message: OrderStatus
        let mut msg2 = ResponseMessage::from("3\0456\0Filled\0");
        handler.parse_account_info(&mut msg2, Some(&callback)).unwrap();

        // Third message: NextValidId (should NOT trigger callback)
        let mut msg3 = ResponseMessage::from("9\01\01000\0");
        handler.parse_account_info(&mut msg3, Some(&callback)).unwrap();

        assert_eq!(*received_count.lock().unwrap(), 2, "callback should be invoked exactly twice");
    }

    #[test]
    fn test_parse_connection_time() {
        let example = "20230405 22:20:39 PST";
        let (connection_time, _) = parse_connection_time(example);

        let la = timezones::db::america::LOS_ANGELES;
        if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
            assert_eq!(connection_time, Some(other));
        }
    }

    #[test]
    fn test_parse_connection_time_china_standard_time() {
        let example = "20230405 22:20:39 China Standard Time";
        let (connection_time, timezone) = parse_connection_time(example);

        assert!(connection_time.is_some());
        assert!(timezone.is_some());
        assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
    }

    #[test]
    fn test_parse_connection_time_chinese_utf8() {
        let example = "20230405 22:20:39 中国标准时间";
        let (connection_time, timezone) = parse_connection_time(example);

        assert!(connection_time.is_some());
        assert!(timezone.is_some());
        assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
    }

    #[test]
    fn test_parse_connection_time_mojibake() {
        // Simulate GB2312 timezone decoded as UTF-8 lossy
        let example = "20230405 22:20:39 \u{FFFD}\u{FFFD}\u{FFFD}";
        let (connection_time, timezone) = parse_connection_time(example);

        assert!(connection_time.is_some());
        assert!(timezone.is_some());
        assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
    }

    #[test]
    fn test_connection_handler_handshake() {
        let handler = ConnectionHandler::default();
        let handshake = handler.format_handshake();

        // Should start with "API\0"
        assert_eq!(&handshake[0..4], b"API\0");

        // Should contain version string
        let version_part = &handshake[4..];
        assert!(!version_part.is_empty());
    }

    #[test]
    fn test_connection_handler_start_api() {
        use crate::messages::PROTOBUF_MSG_ID;

        let handler = ConnectionHandler::default();
        let data = handler.format_start_api(123, server_versions::PROTOBUF);

        // First 4 bytes: big-endian (StartApi=71 + 200)
        let msg_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(msg_id, 71 + PROTOBUF_MSG_ID);

        // Remaining bytes: protobuf-encoded StartApiRequest with client_id=123
        let request: crate::proto::StartApiRequest = prost::Message::decode(&data[4..]).unwrap();
        assert_eq!(request.client_id, Some(123));
    }

    #[test]
    fn test_connection_handler_start_api_legacy() {
        let handler = ConnectionHandler::default();
        let data = handler.format_start_api(123, 150); // server < PROTOBUF

        let text = String::from_utf8(data).unwrap();
        assert!(text.contains("71")); // StartApi message type
        assert!(text.contains("123")); // client_id
    }

    #[test]
    fn test_parse_raw_message_protobuf() {
        use crate::messages::PROTOBUF_MSG_ID;

        // Simulate a protobuf message: msg_id = 5 + 200 = 205, then some payload
        let msg_id: i32 = 5 + PROTOBUF_MSG_ID;
        let payload = vec![0x08, 0x64]; // varint tag=1, value=100
        let mut data = msg_id.to_be_bytes().to_vec();
        data.extend_from_slice(&payload);

        let (message, trace_str) = parse_raw_message(&data, server_versions::PROTOBUF);
        assert!(message.is_protobuf);
        assert_eq!(message.message_type(), IncomingMessages::OpenOrder);
        assert_eq!(message.raw_bytes(), Some(payload.as_slice()));
        assert!(trace_str.is_none()); // no trace string for protobuf
    }

    #[test]
    fn test_parse_raw_message_binary_id_text_payload() {
        // Simulate a text message at server >= 201: binary msg_id=9, then NUL-delimited text
        let msg_id: i32 = 9; // NextValidId
        let text_payload = b"1\01000\0";
        let mut data = msg_id.to_be_bytes().to_vec();
        data.extend_from_slice(text_payload);

        let (message, trace_str) = parse_raw_message(&data, server_versions::PROTOBUF);
        assert!(!message.is_protobuf);
        assert_eq!(message.message_type(), IncomingMessages::NextValidId);
        assert_eq!(message.peek_string(1), "1"); // version field
        assert_eq!(message.peek_int(2).unwrap(), 1000); // next_order_id
        assert!(trace_str.is_some());
    }

    #[test]
    fn test_parse_raw_message_legacy_text() {
        let data = b"9\01\01000\0";
        let (message, trace_str) = parse_raw_message(data, 173); // server < PROTOBUF

        assert!(!message.is_protobuf);
        assert_eq!(message.message_type(), IncomingMessages::NextValidId);
        assert!(trace_str.is_some());
    }

    /// Test handling of non-UTF8 encoded data from IB Gateway (issue #352)
    /// Some IB Gateway installations send timezone names in GB2312/GBK encoding
    /// (e.g., Chinese "中国标准时间" for "China Standard Time")
    #[test]
    fn test_non_utf8_handshake_response() {
        // Actual bytes from issue #352: "173\020251205 23:13:45 中国标准时间\0"
        // where the Chinese characters are GB2312 encoded, not UTF-8
        let gb2312_bytes: Vec<u8> = vec![
            49, 55, 51, 0, // "173\0" - server version
            50, 48, 50, 53, 49, 50, 48, 53, 32, // "20251205 " - date
            50, 51, 58, 49, 51, 58, 52, 53, 32, // "23:13:45 " - time
            214, 208, 185, 250, 177, 234, 215, 188, 202, 177, 188, 228, // GB2312: 中国标准时间
            0,   // null terminator
        ];

        // from_utf8_lossy should handle this without error
        let raw_string = String::from_utf8_lossy(&gb2312_bytes).into_owned();

        // Should contain the ASCII portions intact
        assert!(raw_string.contains("173"));
        assert!(raw_string.contains("20251205"));
        assert!(raw_string.contains("23:13:45"));

        // Non-UTF8 bytes are replaced with replacement character
        assert!(raw_string.contains('\u{FFFD}'));

        // Parse as ResponseMessage and extract handshake data
        let mut response = ResponseMessage::from(&raw_string);
        let handler = ConnectionHandler::default();
        let result = handler.parse_handshake_response(&mut response);

        assert!(result.is_ok());
        let handshake_data = result.unwrap();
        assert_eq!(handshake_data.server_version, 173);
        // server_time will contain replacement characters but parsing succeeds
        assert!(handshake_data.server_time.contains("20251205"));
    }

    #[test]
    fn test_connection_options_default() {
        let opts = ConnectionOptions::default();
        assert_eq!(opts.tcp_no_delay, false);
        assert!(opts.startup_callback.is_none());
    }

    #[test]
    fn test_connection_options_builder() {
        let opts = ConnectionOptions::default().tcp_no_delay(true).startup_callback(|_msg| {});
        assert_eq!(opts.tcp_no_delay, true);
        assert!(opts.startup_callback.is_some());
    }

    #[test]
    fn test_connection_options_clone() {
        let opts = ConnectionOptions::default().tcp_no_delay(true);
        let cloned = opts.clone();
        assert_eq!(cloned.tcp_no_delay, true);
    }

    #[test]
    fn test_connection_options_debug() {
        let opts = ConnectionOptions::default().tcp_no_delay(true);
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("tcp_no_delay: true"));
        assert!(debug_str.contains("startup_callback: false"));
    }
}
