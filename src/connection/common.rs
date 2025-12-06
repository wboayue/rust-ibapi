//! Common connection logic shared between sync and async implementations

use log::{debug, error, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::errors::Error;
use crate::messages::{encode_length, IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::server_versions;

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

    /// Format the start API message
    fn format_start_api(&self, client_id: i32, server_version: i32) -> RequestMessage;

    /// Parse account information from incoming messages
    fn parse_account_info(&self, message: &mut ResponseMessage) -> Result<AccountInfo, Self::Error>;
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
            min_version: 100,
            max_version: server_versions::WSH_EVENT_DATA_FILTERS_DATE,
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

    fn format_start_api(&self, client_id: i32, server_version: i32) -> RequestMessage {
        const VERSION: i32 = 2;

        let mut message = RequestMessage::default();
        message.push_field(&OutgoingMessages::StartApi);
        message.push_field(&VERSION);
        message.push_field(&client_id);

        if server_version > server_versions::OPTIONAL_CAPABILITIES {
            message.push_field(&"");
        }

        message
    }

    fn parse_account_info(&self, message: &mut ResponseMessage) -> Result<AccountInfo, Self::Error> {
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
                error!("Error during account info: {message:?}");
            }
            _ => {
                // Other messages during connection are logged but not processed
                warn!(
                    "CONSUMING MESSAGE during connection setup: {:?} - THIS MESSAGE IS LOST!",
                    message.message_type()
                );
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

    let zones = timezones::find_by_name(parts[2]);

    if zones.is_empty() {
        error!("Time zone not found for {}", parts[2]);
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
        let handler = ConnectionHandler::default();
        let message = handler.format_start_api(123, 150);

        let encoded = message.encode();
        assert!(encoded.contains("71")); // StartApi message type
        assert!(encoded.contains("123")); // client_id
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
}
