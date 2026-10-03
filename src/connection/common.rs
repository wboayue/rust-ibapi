//! Common connection logic shared between sync and async implementations

use log::{debug, error, warn};
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::Tz;

use crate::accounts::AccountUpdate;
use crate::common::timezone::{find_timezone, resolve_local};
use crate::errors::Error;
use crate::messages::{
    encode_length, encode_protobuf_message, unknown_message_type_notice, IncomingMessages, Notice, OutgoingMessages, ResponseMessage,
    HANDSHAKE_DECODE_FAILURE_CODE, HANDSHAKE_UNKNOWN_FRAME_CODE, MESSAGE_ID_LEN, PROTOBUF_MSG_ID,
};
use crate::orders::{CommissionReport, ExecutionData, OrderData, OrderStatus};
use crate::server_versions;
use crate::transport::common::NoticeSink;

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
/// [`HANDSHAKE_DECODE_FAILURE_CODE`](crate::HANDSHAKE_DECODE_FAILURE_CODE). A
/// frame whose message id maps to no kind raises
/// [`UNKNOWN_MESSAGE_TYPE_CODE`](crate::UNKNOWN_MESSAGE_TYPE_CODE).
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
            min_version: server_versions::PROTOBUF_REST_MESSAGES_3,
            max_version: server_versions::ODD_LOT_BID_ASK_QUOTES,
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
        use crate::proto::decoders::DecodeProto;

        let mut info = AccountInfo::default();

        match message.message_type() {
            IncomingMessages::NextValidId => {
                let proto = crate::proto::NextValidId::decode_proto(message.require_proto()?)?;
                info.next_order_id = proto.order_id;
            }
            IncomingMessages::ManagedAccounts => {
                let proto = crate::proto::ManagedAccounts::decode_proto(message.require_proto()?)?;
                info.managed_accounts = proto.accounts_list;
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
/// Decode failures, recognized kinds with no typed variant, and unrecognized
/// message ids route to the notice sink with synthesized codes
/// ([`HANDSHAKE_DECODE_FAILURE_CODE`], [`HANDSHAKE_UNKNOWN_FRAME_CODE`] and
/// [`UNKNOWN_MESSAGE_TYPE_CODE`](crate::UNKNOWN_MESSAGE_TYPE_CODE)) so observers via
/// [`Client::notice_stream`](crate::Client::notice_stream) can detect them.
pub(crate) fn dispatch_unsolicited_message(_server_version: i32, message: &mut ResponseMessage, ctx: &StartupHandshakeContext<'_>) {
    use crate::accounts::common::decode_account_update_message;
    use crate::orders::common::{decode_commission_report, decode_completed_order, decode_execution_data, decode_open_order, decode_order_status};

    /// Run a typed decoder; fire the callback with the typed payload on
    /// success, or emit a synthesized decode-failure notice on error. The
    /// decoder only runs when a callback is present, but the failure notice
    /// always fires when a decode is attempted.
    fn dispatch_typed<T>(
        ctx: &StartupHandshakeContext<'_>,
        kind: IncomingMessages,
        decode: impl FnOnce() -> Result<T, Error>,
        wrap: impl FnOnce(T) -> StartupMessage,
    ) {
        let Some(cb) = ctx.startup else { return };
        match decode() {
            Ok(t) => cb(wrap(t)),
            Err(e) => ctx.notice_sink.deliver(Notice::synthesized(
                HANDSHAKE_DECODE_FAILURE_CODE,
                format!("handshake decoder failed for {kind:?}: {e}"),
            )),
        }
    }

    /// Fire the typed callback with a unit-marker variant if a callback is
    /// installed. No payload to decode; no notice path.
    fn dispatch_unit(ctx: &StartupHandshakeContext<'_>, msg: StartupMessage) {
        if let Some(cb) = ctx.startup {
            cb(msg);
        }
    }

    let kind = message.message_type();
    match kind {
        IncomingMessages::Error => {
            let notice = Notice::from(&*message);
            notice.log();
            ctx.notice_sink.deliver(notice);
        }
        IncomingMessages::OpenOrder => dispatch_typed(ctx, kind, || decode_open_order(message), StartupMessage::OpenOrder),
        IncomingMessages::OrderStatus => dispatch_typed(ctx, kind, || decode_order_status(message), StartupMessage::OrderStatus),
        IncomingMessages::OpenOrderEnd => dispatch_unit(ctx, StartupMessage::OpenOrderEnd),
        IncomingMessages::AccountValue
        | IncomingMessages::PortfolioValue
        | IncomingMessages::AccountUpdateTime
        | IncomingMessages::AccountDownloadEnd => dispatch_typed(ctx, kind, || decode_account_update_message(message), StartupMessage::AccountUpdate),
        IncomingMessages::ExecutionData => dispatch_typed(ctx, kind, || decode_execution_data(message), StartupMessage::Execution),
        IncomingMessages::CommissionsReport => dispatch_typed(ctx, kind, || decode_commission_report(message), StartupMessage::CommissionReport),
        IncomingMessages::CompletedOrder => dispatch_typed(ctx, kind, || decode_completed_order(message), StartupMessage::CompletedOrder),
        IncomingMessages::ExecutionDataEnd => dispatch_unit(ctx, StartupMessage::ExecutionDataEnd),
        IncomingMessages::CompletedOrdersEnd => dispatch_unit(ctx, StartupMessage::CompletedOrdersEnd),
        // An id that maps to no kind is the shape a framing slip takes; it
        // raises the same code here as in steady-state routing, since a
        // reconnect runs through this window.
        IncomingMessages::NotValid => ctx.notice_sink.deliver(unknown_message_type_notice(message)),
        _ => {
            // Recognized kind with no typed variant: log + emit synthesized
            // notice. Fires regardless of callback presence (no typed variant
            // to receive).
            warn!("unrouted handshake frame: {kind:?}");
            ctx.notice_sink.deliver(Notice::synthesized(
                HANDSHAKE_UNKNOWN_FRAME_CODE,
                format!("unsolicited handshake frame with no typed variant: {kind:?}"),
            ));
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
    if server_version < server_versions::PROTOBUF_REST_MESSAGES_3 {
        return Err(Error::ServerVersion(
            server_versions::PROTOBUF_REST_MESSAGES_3,
            server_version,
            format!(
                "protobuf transport — rust-ibapi 3.x requires TWS or IB Gateway with server version {} or later; please upgrade",
                server_versions::PROTOBUF_REST_MESSAGES_3
            ),
        ));
    }
    Ok(())
}

/// Parse connection time from TWS format
/// Format: "20230105 22:20:39 PST"
///
/// Never fails the handshake. A truncated string, an unparseable date or a
/// timezone name that no alias or IANA zone matches yields `None` for the
/// affected component; the unmatched name is logged with how to map it.
pub fn parse_connection_time(connection_time: &str) -> (Option<OffsetDateTime>, Option<&'static Tz>) {
    // The zone is everything after the time and may contain spaces ("China Standard Time").
    let mut parts = connection_time.splitn(3, ' ');
    let (Some(date), Some(time), Some(tz_name)) = (parts.next(), parts.next(), parts.next()) else {
        error!("Invalid connection time format: {connection_time}");
        return (None, None);
    };

    let Some(timezone) = find_timezone(tz_name) else {
        warn!("{}", Error::UnsupportedTimeZone(tz_name.to_string()));
        return (None, None);
    };

    let format = format_description!("[year][month][day] [hour]:[minute]:[second]");
    let date_str = format!("{date} {time}");
    let date = time::PrimitiveDateTime::parse(date_str.as_str(), format);

    match date {
        Ok(connected_at) => (Some(resolve_local(connected_at, timezone)), Some(timezone)),
        Err(err) => {
            warn!("Could not parse connection time from {date_str}: {err}");
            (None, Some(timezone))
        }
    }
}

/// Parse raw message bytes into a `ResponseMessage`.
///
/// Every message frame is `[4-byte BE msg_id][payload]`. When the 4-byte
/// binary message ID exceeds [`PROTOBUF_MSG_ID`], the payload is
/// protobuf-encoded; otherwise it is NUL-delimited text. At floor 213 the
/// text branch is unreachable through production decoders (WSH metadata/
/// event-data come through it via tests; a TWS-emitted text frame for a type
/// with a proto decoder raises `Error::UnexpectedWireFormat`, and one with no
/// decoder at all falls through to the dispatcher catch-all and is skipped).
///
/// A body too short to hold the message id yields [`Error::InvalidFrame`]. The
/// frame readers reject this length before allocating, so production callers
/// never see it; the guard stays because this is also reached from in-memory
/// stream fixtures, which supply bodies directly and skip the length prefix
/// entirely. It used to index straight past the end and panic the dispatcher.
pub fn parse_raw_message(data: &[u8]) -> Result<ResponseMessage, Error> {
    let Some((header, payload)) = data.split_first_chunk::<MESSAGE_ID_LEN>() else {
        return Err(Error::InvalidFrame(format!(
            "frame body of {} bytes cannot hold a message id",
            data.len()
        )));
    };
    let msg_id = i32::from_be_bytes(*header);

    if msg_id > PROTOBUF_MSG_ID {
        let real_type = msg_id - PROTOBUF_MSG_ID;
        debug!("<- protobuf msg_id={real_type}");
        Ok(ResponseMessage::from_protobuf(real_type, payload.to_vec()))
    } else {
        // Binary message ID, NUL-delimited text payload.
        let raw = String::from_utf8_lossy(payload);
        debug!("<- {raw:?}");
        let mut fields = vec![msg_id.to_string()];
        fields.extend(raw.split_terminator('\0').map(|s| s.to_string()));
        Ok(ResponseMessage::from_text_fields(fields))
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
