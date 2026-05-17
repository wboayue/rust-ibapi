//! Common connection logic shared between sync and async implementations

use log::{debug, error, info, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{OffsetResult, PrimitiveDateTimeExt, Tz};

use crate::accounts::AccountUpdate;
use crate::common::timezone::find_timezone;
use crate::errors::Error;
use crate::messages::{
    encode_length, encode_protobuf_message, IncomingMessages, Notice, OutgoingMessages, ResponseMessage, HANDSHAKE_DECODE_FAILURE_CODE,
    HANDSHAKE_UNKNOWN_FRAME_CODE, PROTOBUF_MSG_ID,
};
use crate::orders::{CommissionReport, ExecutionData, OrderData, OrderStatus};
use crate::server_versions;

/// Domain-typed messages delivered to the startup callback during the
/// connection handshake (initial connect *and* auto-reconnect).
///
/// TWS may emit any of these unsolicited at handshake time when the connection
/// is bound to the configured Master Client ID (open-order + commission-report
/// replays), or when the previous session left outstanding orders / account
/// state worth resending. Frame kinds with no typed variant — and frames whose
/// typed decoder fails — are routed to the notice stream
/// ([`Client::notice_stream`](crate::Client::notice_stream)) instead, using
/// the synthesized codes
/// [`HANDSHAKE_UNKNOWN_FRAME_CODE`](crate::HANDSHAKE_UNKNOWN_FRAME_CODE) and
/// [`HANDSHAKE_DECODE_FAILURE_CODE`](crate::HANDSHAKE_DECODE_FAILURE_CODE).
#[derive(Debug)]
#[non_exhaustive]
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
    /// Execution detail — TWS replays prior fills to the Master Client ID
    /// after `start_api` when the previous session bound them.
    Execution(ExecutionData),
    /// Commission and fees report — TWS replays the per-fill commission to
    /// the Master Client ID alongside [`Execution`](Self::Execution).
    CommissionReport(CommissionReport),
    /// Completed (terminal-state) order — TWS replays the closed-order history
    /// at handshake time when a prior session requested
    /// `reqCompletedOrders`. The contained [`OrderData::order_id`] is the
    /// legacy sentinel `-1` (no live order id for completed orders).
    CompletedOrder(OrderData),
    /// End-of-executions marker. No payload.
    ExecutionDataEnd,
    /// End-of-completed-orders marker. No payload.
    CompletedOrdersEnd,
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
            StartupMessage::Execution(_) => IncomingMessages::ExecutionData,
            StartupMessage::CommissionReport(_) => IncomingMessages::CommissionsReport,
            StartupMessage::CompletedOrder(_) => IncomingMessages::CompletedOrder,
            StartupMessage::ExecutionDataEnd => IncomingMessages::ExecutionDataEnd,
            StartupMessage::CompletedOrdersEnd => IncomingMessages::CompletedOrdersEnd,
        }
    }
}

/// Build a synthesized notice signaling that a handshake-time frame's
/// `IncomingMessages` kind has no typed [`StartupMessage`] variant. Routed to
/// the notice sink (always present), distinguishable from TWS-emitted notices
/// by [`Notice::is_handshake_synthetic`].
fn synthesized_unknown_frame_notice(kind: IncomingMessages) -> Notice {
    Notice {
        code: HANDSHAKE_UNKNOWN_FRAME_CODE,
        message: format!("unsolicited handshake frame with no typed variant: {kind:?}"),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }
}

/// Build a synthesized notice signaling that a typed handshake decoder failed
/// on a known [`IncomingMessages`] kind.
fn synthesized_decode_failure_notice(kind: IncomingMessages, err: &Error) -> Notice {
    Notice {
        code: HANDSHAKE_DECODE_FAILURE_CODE,
        message: format!("handshake decoder failed for {kind:?}: {err}"),
        error_time: None,
        advanced_order_reject_json: String::new(),
    }
}

/// Sink for unrouted notices observed during the handshake. Production impls
/// forward to the per-feature notice broadcaster owned by `Connection`, so
/// handshake-time notices reach any pre-bound `NoticeStream` the user obtained
/// from `ClientBuilder::connect_with_notice_stream`.
pub(crate) trait NoticeSink: Send + Sync {
    fn deliver(&self, notice: Notice);
}

#[cfg(feature = "sync")]
impl NoticeSink for crate::transport::sync::NoticeBroadcaster {
    fn deliver(&self, notice: Notice) {
        self.broadcast(notice);
    }
}

#[cfg(feature = "async")]
impl NoticeSink for tokio::sync::broadcast::Sender<Notice> {
    fn deliver(&self, notice: Notice) {
        let _ = self.send(notice);
    }
}

/// Handshake-time context bundling the optional typed-message callback and the
/// mandatory notice sink. Internal use only; the public surface is
/// `ClientBuilder` (`crate::client::ClientBuilder` for async,
/// `crate::client::blocking::ClientBuilder` for sync).
pub(crate) struct StartupHandshakeContext<'a> {
    pub startup: Option<&'a (dyn Fn(StartupMessage) + Send + Sync)>,
    pub notice_sink: &'a (dyn NoticeSink + Sync),
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
        ctx: &StartupHandshakeContext<'_>,
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
            min_version: server_versions::PROTOBUF_SCAN_DATA,
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
        ctx: &StartupHandshakeContext<'_>,
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
            _ => dispatch_unsolicited_message(server_version, message, ctx),
        }

        Ok(info)
    }
}

/// Dispatch an unsolicited message that arrived during the handshake — i.e. one
/// that wasn't `NextValidId` / `ManagedAccounts` (which `parse_account_info`
/// consumes itself). Errors fan out to the notice sink (always present);
/// typed frames (`OpenOrder` / `OrderStatus` / account-update / execution /
/// commission / completed-order, plus the corresponding end markers) decode
/// into typed [`StartupMessage`] values for the optional startup callback.
/// Decode failures and unknown frame kinds route to the notice sink with
/// synthesized codes ([`HANDSHAKE_DECODE_FAILURE_CODE`] and
/// [`HANDSHAKE_UNKNOWN_FRAME_CODE`]) so observers via
/// [`Client::notice_stream`](crate::Client::notice_stream) can detect them.
pub(crate) fn dispatch_unsolicited_message(server_version: i32, message: &mut ResponseMessage, ctx: &StartupHandshakeContext<'_>) {
    use crate::accounts::common::decode_account_update_message;
    use crate::orders::common::{decode_commission_report, decode_completed_order, decode_execution_data, decode_open_order, decode_order_status};

    /// Local helper: run a typed decoder and either fire the callback with
    /// the typed payload, or emit a synthesized decode-failure notice. The
    /// decoder only runs when a callback is present (matches the previous
    /// behavior — nothing else consumes the typed payload), but the failure
    /// notice always fires when a decode is attempted.
    fn dispatch_typed<T>(
        ctx: &StartupHandshakeContext<'_>,
        kind: IncomingMessages,
        decode: impl FnOnce() -> Result<T, Error>,
        wrap: impl FnOnce(T) -> StartupMessage,
    ) {
        let Some(cb) = ctx.startup else { return };
        match decode() {
            Ok(t) => cb(wrap(t)),
            Err(e) => ctx.notice_sink.deliver(synthesized_decode_failure_notice(kind, &e)),
        }
    }

    let kind = message.message_type();
    match kind {
        IncomingMessages::Error => {
            let notice = Notice::from(&*message);
            if notice.is_warning() || notice.is_system_message() {
                info!("{notice}");
            } else {
                error!("Error during account info: {notice}");
            }
            ctx.notice_sink.deliver(notice);
        }
        IncomingMessages::OpenOrder => dispatch_typed(ctx, kind, || decode_open_order(message), StartupMessage::OpenOrder),
        IncomingMessages::OrderStatus => dispatch_typed(ctx, kind, || decode_order_status(message), StartupMessage::OrderStatus),
        IncomingMessages::OpenOrderEnd => {
            if let Some(cb) = ctx.startup {
                cb(StartupMessage::OpenOrderEnd);
            }
        }
        IncomingMessages::AccountValue
        | IncomingMessages::PortfolioValue
        | IncomingMessages::AccountUpdateTime
        | IncomingMessages::AccountDownloadEnd => dispatch_typed(
            ctx,
            kind,
            || decode_account_update_message(server_version, message),
            StartupMessage::AccountUpdate,
        ),
        IncomingMessages::ExecutionData => dispatch_typed(ctx, kind, || decode_execution_data(message), StartupMessage::Execution),
        IncomingMessages::CommissionsReport => dispatch_typed(ctx, kind, || decode_commission_report(message), StartupMessage::CommissionReport),
        IncomingMessages::CompletedOrder => dispatch_typed(ctx, kind, || decode_completed_order(message), StartupMessage::CompletedOrder),
        IncomingMessages::ExecutionDataEnd => {
            if let Some(cb) = ctx.startup {
                cb(StartupMessage::ExecutionDataEnd);
            }
        }
        IncomingMessages::CompletedOrdersEnd => {
            if let Some(cb) = ctx.startup {
                cb(StartupMessage::CompletedOrdersEnd);
            }
        }
        _ => {
            // Genuinely-unknown frame kind: log + emit synthesized notice.
            // Always fires (callback presence is irrelevant — the callback
            // has no typed variant to receive anyway).
            warn!("unrouted handshake frame: {kind:?}");
            ctx.notice_sink.deliver(synthesized_unknown_frame_notice(kind));
        }
    }
}

/// Reject connections to TWS/IB Gateway builds older than the protobuf transport.
///
/// rust-ibapi 3.x is protobuf-only; `start_api` and every request encoder emit
/// protobuf, so a server below the floor cannot interpret what we send. The
/// floor ratchets up alongside the per-family text→proto migration; bumping it
/// is what lets us delete the now-unreachable text-decoder branches in each
/// domain. Fail fast after the handshake with a descriptive error rather than
/// letting the gateway silently drop our messages.
pub(crate) fn require_protobuf_support(server_version: i32) -> Result<(), Error> {
    if server_version < server_versions::PROTOBUF_SCAN_DATA {
        return Err(Error::ServerVersion(
            server_versions::PROTOBUF_SCAN_DATA,
            server_version,
            format!(
                "protobuf transport — rust-ibapi 3.x requires TWS or IB Gateway with server version {} or later; please upgrade",
                server_versions::PROTOBUF_SCAN_DATA
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
