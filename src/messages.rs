//! Message encoding, decoding, and routing for TWS API communication.
//!
//! This module handles the low-level message protocol between the client and TWS,
//! including request/response message formatting, field encoding/decoding,
//! and message type definitions.

use std::fmt::Display;
use std::io::Write;
use std::str::{self, FromStr};

use byteorder::{BigEndian, WriteBytesExt};

use log::debug;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{Error, ToField};

pub mod parser_registry;
pub(crate) mod shared_channel_configuration;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod from_str_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_outgoing_messages_from_str() {
        // Test some common message types
        assert_eq!(OutgoingMessages::from_str("1").unwrap(), OutgoingMessages::RequestMarketData);
        assert_eq!(OutgoingMessages::from_str("17").unwrap(), OutgoingMessages::RequestManagedAccounts);
        assert_eq!(OutgoingMessages::from_str("49").unwrap(), OutgoingMessages::RequestCurrentTime);
        assert_eq!(OutgoingMessages::from_str("61").unwrap(), OutgoingMessages::RequestPositions);

        // Test error cases
        assert!(OutgoingMessages::from_str("999").is_err());
        assert!(OutgoingMessages::from_str("abc").is_err());
        assert!(OutgoingMessages::from_str("").is_err());
    }

    #[test]
    fn test_outgoing_messages_roundtrip() {
        // Test that we can convert to string and back
        let msg = OutgoingMessages::RequestCurrentTime;
        let as_string = msg.to_string();
        let parsed = OutgoingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, OutgoingMessages::RequestCurrentTime);

        // Test with another message type
        let msg = OutgoingMessages::RequestManagedAccounts;
        let as_string = msg.to_string();
        let parsed = OutgoingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, OutgoingMessages::RequestManagedAccounts);
    }

    #[test]
    fn test_incoming_messages_from_str() {
        // Test some common message types
        assert_eq!(IncomingMessages::from_str("4").unwrap(), IncomingMessages::Error);
        assert_eq!(IncomingMessages::from_str("15").unwrap(), IncomingMessages::ManagedAccounts);
        assert_eq!(IncomingMessages::from_str("49").unwrap(), IncomingMessages::CurrentTime);
        assert_eq!(IncomingMessages::from_str("61").unwrap(), IncomingMessages::Position);

        // Test NotValid for unknown values
        assert_eq!(IncomingMessages::from_str("999").unwrap(), IncomingMessages::NotValid);
        assert_eq!(IncomingMessages::from_str("0").unwrap(), IncomingMessages::NotValid);
        assert_eq!(IncomingMessages::from_str("-1").unwrap(), IncomingMessages::NotValid);

        // Test error cases for non-numeric strings
        assert!(IncomingMessages::from_str("abc").is_err());
        assert!(IncomingMessages::from_str("").is_err());
        assert!(IncomingMessages::from_str("1.5").is_err());
    }

    #[test]
    fn test_incoming_messages_roundtrip() {
        // Test with CurrentTime message
        let n = 49;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);

        // Test with ManagedAccounts message
        let n = 15;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);

        // Test with NotValid (unknown value)
        let n = 999;
        let msg = IncomingMessages::from(n);
        let as_string = n.to_string();
        let parsed = IncomingMessages::from_str(&as_string).unwrap();
        assert_eq!(parsed, msg);
        assert_eq!(parsed, IncomingMessages::NotValid);
    }
}

/// Offset added to outbound protobuf message IDs. Inbound IDs > this value are protobuf.
pub(crate) const PROTOBUF_MSG_ID: i32 = 200;

/// Messages emitted by TWS/Gateway over the market data socket.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum IncomingMessages {
    /// Gateway initiated shutdown.
    Shutdown = -2,
    /// Unknown or unsupported message id.
    NotValid = -1,
    /// Tick price update.
    TickPrice = 1,
    /// Tick size update.
    TickSize = 2,
    /// Order status update.
    OrderStatus = 3,
    /// Error (includes request id and code).
    Error = 4,
    /// Open order description.
    OpenOrder = 5,
    /// Account value key/value pair.
    AccountValue = 6,
    /// Portfolio value line.
    PortfolioValue = 7,
    /// Account update timestamp.
    AccountUpdateTime = 8,
    /// Next valid order id notification.
    NextValidId = 9,
    /// Contract details payload.
    ContractData = 10,
    /// Execution data update.
    ExecutionData = 11,
    /// Level 1 market depth row update.
    MarketDepth = 12,
    /// Level 2 market depth row update.
    MarketDepthL2 = 13,
    /// News bulletin broadcast.
    NewsBulletins = 14,
    /// List of managed accounts.
    ManagedAccounts = 15,
    /// Financial advisor configuration data.
    ReceiveFA = 16,
    /// Historical bar data payload.
    HistoricalData = 17,
    /// Bond contract details payload.
    BondContractData = 18,
    /// Scanner parameter definitions.
    ScannerParameters = 19,
    /// Scanner subscription results.
    ScannerData = 20,
    /// Option computation tick.
    TickOptionComputation = 21,
    /// Generic numeric tick (e.g. implied volatility).
    TickGeneric = 45,
    /// String-valued tick (exchange names, etc.).
    TickString = 46,
    /// Exchange for Physical tick update.
    TickEFP = 47, //TICK EFP 47
    /// Current world clock time.
    CurrentTime = 49,
    /// Real-time bars update.
    RealTimeBars = 50,
    /// Fundamental data response.
    FundamentalData = 51,
    /// End marker for contract details batches.
    ContractDataEnd = 52,
    /// End marker for open order batches.
    OpenOrderEnd = 53,
    /// End marker for account download.
    AccountDownloadEnd = 54,
    /// End marker for execution data.
    ExecutionDataEnd = 55,
    /// Delta-neutral validation response.
    DeltaNeutralValidation = 56,
    /// End of tick snapshot.
    TickSnapshotEnd = 57,
    /// Market data type acknowledgment.
    MarketDataType = 58,
    /// Commissions report payload.
    CommissionsReport = 59,
    /// Position update.
    Position = 61,
    /// End marker for position updates.
    PositionEnd = 62,
    /// Account summary update.
    AccountSummary = 63,
    /// End marker for account summary stream.
    AccountSummaryEnd = 64,
    /// API verification challenge.
    VerifyMessageApi = 65,
    /// API verification completion.
    VerifyCompleted = 66,
    /// Display group list response.
    DisplayGroupList = 67,
    /// Display group update.
    DisplayGroupUpdated = 68,
    /// Auth + verification challenge.
    VerifyAndAuthMessageApi = 69,
    /// Auth + verification completion.
    VerifyAndAuthCompleted = 70,
    /// Multi-account position update.
    PositionMulti = 71,
    /// End marker for multi-account position stream.
    PositionMultiEnd = 72,
    /// Multi-account account update.
    AccountUpdateMulti = 73,
    /// End marker for multi-account account stream.
    AccountUpdateMultiEnd = 74,
    /// Option security definition parameters.
    SecurityDefinitionOptionParameter = 75,
    /// End marker for option security definition stream.
    SecurityDefinitionOptionParameterEnd = 76,
    /// Soft dollar tier information.
    SoftDollarTier = 77,
    /// Family code response.
    FamilyCodes = 78,
    /// Matching symbol samples.
    SymbolSamples = 79,
    /// Exchanges offering market depth.
    MktDepthExchanges = 80,
    /// Tick request parameter info.
    TickReqParams = 81,
    /// Smart component routing map.
    SmartComponents = 82,
    /// News article content.
    NewsArticle = 83,
    /// News headline tick.
    TickNews = 84,
    /// Available news providers.
    NewsProviders = 85,
    /// Historical news headlines.
    HistoricalNews = 86,
    /// End marker for historical news.
    HistoricalNewsEnd = 87,
    /// Head timestamp for historical data.
    HeadTimestamp = 88,
    /// Histogram data response.
    HistogramData = 89,
    /// Streaming historical data update.
    HistoricalDataUpdate = 90,
    /// Market data request reroute notice.
    RerouteMktDataReq = 91,
    /// Market depth request reroute notice.
    RerouteMktDepthReq = 92,
    /// Market rule response.
    MarketRule = 93,
    /// Account PnL update.
    PnL = 94,
    /// Single position PnL update.
    PnLSingle = 95,
    /// Historical tick data (midpoint).
    HistoricalTick = 96,
    /// Historical tick data (bid/ask).
    HistoricalTickBidAsk = 97,
    /// Historical tick data (trades).
    HistoricalTickLast = 98,
    /// Tick-by-tick streaming data.
    TickByTick = 99,
    /// Order bound notification for API multiple endpoints.
    OrderBound = 100,
    /// Completed order information.
    CompletedOrder = 101,
    /// End marker for completed orders.
    CompletedOrdersEnd = 102,
    /// End marker for FA profile replacement.
    ReplaceFAEnd = 103,
    /// Wall Street Horizon metadata update.
    WshMetaData = 104,
    /// Wall Street Horizon event payload.
    WshEventData = 105,
    /// Historical schedule response.
    HistoricalSchedule = 106,
    /// User information response.
    UserInfo = 107,
    /// End marker for historical data.
    HistoricalDataEnd = 108,
    /// Current time in milliseconds.
    CurrentTimeInMillis = 109,
    /// Configuration response.
    ConfigResponse = 110,
    /// Update configuration response.
    UpdateConfigResponse = 111,
}

impl From<i32> for IncomingMessages {
    fn from(value: i32) -> IncomingMessages {
        match value {
            -2 => IncomingMessages::Shutdown,
            1 => IncomingMessages::TickPrice,
            2 => IncomingMessages::TickSize,
            3 => IncomingMessages::OrderStatus,
            4 => IncomingMessages::Error,
            5 => IncomingMessages::OpenOrder,
            6 => IncomingMessages::AccountValue,
            7 => IncomingMessages::PortfolioValue,
            8 => IncomingMessages::AccountUpdateTime,
            9 => IncomingMessages::NextValidId,
            10 => IncomingMessages::ContractData,
            11 => IncomingMessages::ExecutionData,
            12 => IncomingMessages::MarketDepth,
            13 => IncomingMessages::MarketDepthL2,
            14 => IncomingMessages::NewsBulletins,
            15 => IncomingMessages::ManagedAccounts,
            16 => IncomingMessages::ReceiveFA,
            17 => IncomingMessages::HistoricalData,
            18 => IncomingMessages::BondContractData,
            19 => IncomingMessages::ScannerParameters,
            20 => IncomingMessages::ScannerData,
            21 => IncomingMessages::TickOptionComputation,
            45 => IncomingMessages::TickGeneric,
            46 => IncomingMessages::TickString,
            47 => IncomingMessages::TickEFP, //TICK EFP 47
            49 => IncomingMessages::CurrentTime,
            50 => IncomingMessages::RealTimeBars,
            51 => IncomingMessages::FundamentalData,
            52 => IncomingMessages::ContractDataEnd,
            53 => IncomingMessages::OpenOrderEnd,
            54 => IncomingMessages::AccountDownloadEnd,
            55 => IncomingMessages::ExecutionDataEnd,
            56 => IncomingMessages::DeltaNeutralValidation,
            57 => IncomingMessages::TickSnapshotEnd,
            58 => IncomingMessages::MarketDataType,
            59 => IncomingMessages::CommissionsReport,
            61 => IncomingMessages::Position,
            62 => IncomingMessages::PositionEnd,
            63 => IncomingMessages::AccountSummary,
            64 => IncomingMessages::AccountSummaryEnd,
            65 => IncomingMessages::VerifyMessageApi,
            66 => IncomingMessages::VerifyCompleted,
            67 => IncomingMessages::DisplayGroupList,
            68 => IncomingMessages::DisplayGroupUpdated,
            69 => IncomingMessages::VerifyAndAuthMessageApi,
            70 => IncomingMessages::VerifyAndAuthCompleted,
            71 => IncomingMessages::PositionMulti,
            72 => IncomingMessages::PositionMultiEnd,
            73 => IncomingMessages::AccountUpdateMulti,
            74 => IncomingMessages::AccountUpdateMultiEnd,
            75 => IncomingMessages::SecurityDefinitionOptionParameter,
            76 => IncomingMessages::SecurityDefinitionOptionParameterEnd,
            77 => IncomingMessages::SoftDollarTier,
            78 => IncomingMessages::FamilyCodes,
            79 => IncomingMessages::SymbolSamples,
            80 => IncomingMessages::MktDepthExchanges,
            81 => IncomingMessages::TickReqParams,
            82 => IncomingMessages::SmartComponents,
            83 => IncomingMessages::NewsArticle,
            84 => IncomingMessages::TickNews,
            85 => IncomingMessages::NewsProviders,
            86 => IncomingMessages::HistoricalNews,
            87 => IncomingMessages::HistoricalNewsEnd,
            88 => IncomingMessages::HeadTimestamp,
            89 => IncomingMessages::HistogramData,
            90 => IncomingMessages::HistoricalDataUpdate,
            91 => IncomingMessages::RerouteMktDataReq,
            92 => IncomingMessages::RerouteMktDepthReq,
            93 => IncomingMessages::MarketRule,
            94 => IncomingMessages::PnL,
            95 => IncomingMessages::PnLSingle,
            96 => IncomingMessages::HistoricalTick,
            97 => IncomingMessages::HistoricalTickBidAsk,
            98 => IncomingMessages::HistoricalTickLast,
            99 => IncomingMessages::TickByTick,
            100 => IncomingMessages::OrderBound,
            101 => IncomingMessages::CompletedOrder,
            102 => IncomingMessages::CompletedOrdersEnd,
            103 => IncomingMessages::ReplaceFAEnd,
            104 => IncomingMessages::WshMetaData,
            105 => IncomingMessages::WshEventData,
            106 => IncomingMessages::HistoricalSchedule,
            107 => IncomingMessages::UserInfo,
            108 => IncomingMessages::HistoricalDataEnd,
            109 => IncomingMessages::CurrentTimeInMillis,
            110 => IncomingMessages::ConfigResponse,
            111 => IncomingMessages::UpdateConfigResponse,
            _ => IncomingMessages::NotValid,
        }
    }
}

impl FromStr for IncomingMessages {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(n) => Ok(IncomingMessages::from(n)),
            Err(_) => Err(Error::parse_field(s, "invalid incoming message type")),
        }
    }
}

/// Return the message field index containing the request id, if present.
///
/// Post-floor-213, only [`IncomingMessages::TickEFP`] still arrives text-framed
/// from TWS (no protobuf encoder on the server side); every other entry below
/// is dead in production and kept defensively for unsolicited message types
/// that may never be wired.
pub(crate) fn request_id_index(kind: IncomingMessages) -> Option<usize> {
    match kind {
        IncomingMessages::AccountSummary => Some(2),
        IncomingMessages::AccountSummaryEnd => Some(2),
        IncomingMessages::AccountUpdateMulti => Some(2),
        IncomingMessages::AccountUpdateMultiEnd => Some(2),
        IncomingMessages::ContractData => Some(1),
        IncomingMessages::ContractDataEnd => Some(2),
        // Error uses version-dependent indices; use ResponseMessage::error_request_id() instead.
        IncomingMessages::ExecutionData => Some(1),
        IncomingMessages::ExecutionDataEnd => Some(2),
        IncomingMessages::HeadTimestamp => Some(1),
        IncomingMessages::HistogramData => Some(1),
        IncomingMessages::HistoricalData => Some(1),
        IncomingMessages::HistoricalDataEnd => Some(1),
        IncomingMessages::HistoricalDataUpdate => Some(1),
        IncomingMessages::HistoricalNews => Some(1),
        IncomingMessages::HistoricalNewsEnd => Some(1),
        IncomingMessages::HistoricalSchedule => Some(1),
        IncomingMessages::HistoricalTick => Some(1),
        IncomingMessages::HistoricalTickBidAsk => Some(1),
        IncomingMessages::HistoricalTickLast => Some(1),
        IncomingMessages::MarketDepth => Some(2),
        IncomingMessages::MarketDepthL2 => Some(2),
        IncomingMessages::NewsArticle => Some(1),
        IncomingMessages::OpenOrder => Some(1),
        IncomingMessages::PnL => Some(1),
        IncomingMessages::PnLSingle => Some(1),
        IncomingMessages::PositionMulti => Some(2),
        IncomingMessages::PositionMultiEnd => Some(2),
        IncomingMessages::RealTimeBars => Some(2),
        IncomingMessages::ScannerData => Some(2),
        IncomingMessages::SecurityDefinitionOptionParameter => Some(1),
        IncomingMessages::SecurityDefinitionOptionParameterEnd => Some(1),
        IncomingMessages::SymbolSamples => Some(1),
        IncomingMessages::TickByTick => Some(1),
        IncomingMessages::TickEFP => Some(2),
        IncomingMessages::TickGeneric => Some(2),
        IncomingMessages::TickNews => Some(1),
        IncomingMessages::TickOptionComputation => Some(1),
        IncomingMessages::TickPrice => Some(2),
        IncomingMessages::TickReqParams => Some(1),
        IncomingMessages::TickSize => Some(2),
        IncomingMessages::TickSnapshotEnd => Some(2),
        IncomingMessages::TickString => Some(2),
        IncomingMessages::WshEventData => Some(1),
        IncomingMessages::WshMetaData => Some(1),
        IncomingMessages::DisplayGroupList => Some(2),
        IncomingMessages::DisplayGroupUpdated => Some(2),

        _ => {
            debug!("could not determine request id index for {kind:?} (this message type may not have a request id).");
            None
        }
    }
}

/// Outgoing message opcodes understood by TWS/Gateway.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum OutgoingMessages {
    /// Request streaming market data.
    RequestMarketData = 1,
    /// Cancel streaming market data.
    CancelMarketData = 2,
    /// Submit a new order.
    PlaceOrder = 3,
    /// Cancel an existing order.
    CancelOrder = 4,
    /// Request the current open orders.
    RequestOpenOrders = 5,
    /// Request account value updates.
    RequestAccountData = 6,
    /// Request execution reports.
    RequestExecutions = 7,
    /// Request a block of valid order ids.
    RequestIds = 8,
    /// Request contract details.
    RequestContractData = 9,
    /// Request level-two market depth.
    RequestMarketDepth = 10,
    /// Cancel level-two market depth.
    CancelMarketDepth = 11,
    /// Subscribe to news bulletins.
    RequestNewsBulletins = 12,
    /// Cancel news bulletin subscription.
    CancelNewsBulletin = 13,
    /// Change the server log level.
    ChangeServerLog = 14,
    /// Request auto-open orders.
    RequestAutoOpenOrders = 15,
    /// Request all open orders.
    RequestAllOpenOrders = 16,
    /// Request managed accounts list.
    RequestManagedAccounts = 17,
    /// Request financial advisor configuration.
    RequestFA = 18,
    /// Replace financial advisor configuration.
    ReplaceFA = 19,
    /// Request historical bar data.
    RequestHistoricalData = 20,
    /// Exercise an option contract.
    ExerciseOptions = 21,
    /// Subscribe to a market scanner.
    RequestScannerSubscription = 22,
    /// Cancel a market scanner subscription.
    CancelScannerSubscription = 23,
    /// Request scanner parameter definitions.
    RequestScannerParameters = 24,
    /// Cancel an in-flight historical data request.
    CancelHistoricalData = 25,
    /// Request the current TWS/Gateway time.
    RequestCurrentTime = 49,
    /// Request real-time bars.
    RequestRealTimeBars = 50,
    /// Cancel real-time bars.
    CancelRealTimeBars = 51,
    /// Request fundamental data.
    RequestFundamentalData = 52,
    /// Cancel fundamental data.
    CancelFundamentalData = 53,
    /// Request implied volatility calculation.
    ReqCalcImpliedVolat = 54,
    /// Request option price calculation.
    ReqCalcOptionPrice = 55,
    /// Cancel implied volatility calculation.
    CancelImpliedVolatility = 56,
    /// Cancel option price calculation.
    CancelOptionPrice = 57,
    /// Issue a global cancel request.
    RequestGlobalCancel = 58,
    /// Change the active market data type.
    RequestMarketDataType = 59,
    /// Subscribe to position updates.
    RequestPositions = 61,
    /// Subscribe to account summary.
    RequestAccountSummary = 62,
    /// Cancel account summary subscription.
    CancelAccountSummary = 63,
    /// Cancel position subscription.
    CancelPositions = 64,
    /// Begin API verification handshake.
    VerifyRequest = 65,
    /// Respond to verification handshake.
    VerifyMessage = 66,
    /// Query display groups.
    QueryDisplayGroups = 67,
    /// Subscribe to display group events.
    SubscribeToGroupEvents = 68,
    /// Update a display group subscription.
    UpdateDisplayGroup = 69,
    /// Unsubscribe from display group events.
    UnsubscribeFromGroupEvents = 70,
    /// Start the API session.
    StartApi = 71,
    /// Verification handshake with auth.
    VerifyAndAuthRequest = 72,
    /// Verification message with auth.
    VerifyAndAuthMessage = 73,
    /// Request multi-account/model positions.
    RequestPositionsMulti = 74,
    /// Cancel multi-account/model positions.
    CancelPositionsMulti = 75,
    /// Request multi-account/model updates.
    RequestAccountUpdatesMulti = 76,
    /// Cancel multi-account/model updates.
    CancelAccountUpdatesMulti = 77,
    /// Request option security definition parameters.
    RequestSecurityDefinitionOptionalParameters = 78,
    /// Request soft-dollar tier definitions.
    RequestSoftDollarTiers = 79,
    /// Request family codes.
    RequestFamilyCodes = 80,
    /// Request matching symbols.
    RequestMatchingSymbols = 81,
    /// Request exchanges that support depth.
    RequestMktDepthExchanges = 82,
    /// Request smart routing component map.
    RequestSmartComponents = 83,
    /// Request detailed news article.
    RequestNewsArticle = 84,
    /// Request available news providers.
    RequestNewsProviders = 85,
    /// Request historical news headlines.
    RequestHistoricalNews = 86,
    /// Request earliest timestamp for historical data.
    RequestHeadTimestamp = 87,
    /// Request histogram snapshot.
    RequestHistogramData = 88,
    /// Cancel histogram snapshot.
    CancelHistogramData = 89,
    /// Cancel head timestamp request.
    CancelHeadTimestamp = 90,
    /// Request market rule definition.
    RequestMarketRule = 91,
    /// Request account-wide PnL stream.
    RequestPnL = 92,
    /// Cancel account-wide PnL stream.
    CancelPnL = 93,
    /// Request single-position PnL stream.
    RequestPnLSingle = 94,
    /// Cancel single-position PnL stream.
    CancelPnLSingle = 95,
    /// Request historical tick data.
    RequestHistoricalTicks = 96,
    /// Request tick-by-tick data.
    RequestTickByTickData = 97,
    /// Cancel tick-by-tick data.
    CancelTickByTickData = 98,
    /// Request completed order history.
    RequestCompletedOrders = 99,
    /// Request Wall Street Horizon metadata.
    RequestWshMetaData = 100,
    /// Cancel Wall Street Horizon metadata.
    CancelWshMetaData = 101,
    /// Request Wall Street Horizon event data.
    RequestWshEventData = 102,
    /// Cancel Wall Street Horizon event data.
    CancelWshEventData = 103,
    /// Request user information.
    RequestUserInfo = 104,
    /// Request current time in milliseconds.
    RequestCurrentTimeInMillis = 105,
    /// Cancel contract data request.
    CancelContractData = 106,
    /// Cancel historical ticks request.
    CancelHistoricalTicks = 107,
    /// Request configuration.
    ReqConfig = 108,
    /// Update configuration.
    UpdateConfig = 109,
}

impl ToField for OutgoingMessages {
    fn to_field(&self) -> String {
        (*self as i32).to_string()
    }
}

impl std::fmt::Display for OutgoingMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

impl FromStr for OutgoingMessages {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(1) => Ok(OutgoingMessages::RequestMarketData),
            Ok(2) => Ok(OutgoingMessages::CancelMarketData),
            Ok(3) => Ok(OutgoingMessages::PlaceOrder),
            Ok(4) => Ok(OutgoingMessages::CancelOrder),
            Ok(5) => Ok(OutgoingMessages::RequestOpenOrders),
            Ok(6) => Ok(OutgoingMessages::RequestAccountData),
            Ok(7) => Ok(OutgoingMessages::RequestExecutions),
            Ok(8) => Ok(OutgoingMessages::RequestIds),
            Ok(9) => Ok(OutgoingMessages::RequestContractData),
            Ok(10) => Ok(OutgoingMessages::RequestMarketDepth),
            Ok(11) => Ok(OutgoingMessages::CancelMarketDepth),
            Ok(12) => Ok(OutgoingMessages::RequestNewsBulletins),
            Ok(13) => Ok(OutgoingMessages::CancelNewsBulletin),
            Ok(14) => Ok(OutgoingMessages::ChangeServerLog),
            Ok(15) => Ok(OutgoingMessages::RequestAutoOpenOrders),
            Ok(16) => Ok(OutgoingMessages::RequestAllOpenOrders),
            Ok(17) => Ok(OutgoingMessages::RequestManagedAccounts),
            Ok(18) => Ok(OutgoingMessages::RequestFA),
            Ok(19) => Ok(OutgoingMessages::ReplaceFA),
            Ok(20) => Ok(OutgoingMessages::RequestHistoricalData),
            Ok(21) => Ok(OutgoingMessages::ExerciseOptions),
            Ok(22) => Ok(OutgoingMessages::RequestScannerSubscription),
            Ok(23) => Ok(OutgoingMessages::CancelScannerSubscription),
            Ok(24) => Ok(OutgoingMessages::RequestScannerParameters),
            Ok(25) => Ok(OutgoingMessages::CancelHistoricalData),
            Ok(49) => Ok(OutgoingMessages::RequestCurrentTime),
            Ok(50) => Ok(OutgoingMessages::RequestRealTimeBars),
            Ok(51) => Ok(OutgoingMessages::CancelRealTimeBars),
            Ok(52) => Ok(OutgoingMessages::RequestFundamentalData),
            Ok(53) => Ok(OutgoingMessages::CancelFundamentalData),
            Ok(54) => Ok(OutgoingMessages::ReqCalcImpliedVolat),
            Ok(55) => Ok(OutgoingMessages::ReqCalcOptionPrice),
            Ok(56) => Ok(OutgoingMessages::CancelImpliedVolatility),
            Ok(57) => Ok(OutgoingMessages::CancelOptionPrice),
            Ok(58) => Ok(OutgoingMessages::RequestGlobalCancel),
            Ok(59) => Ok(OutgoingMessages::RequestMarketDataType),
            Ok(61) => Ok(OutgoingMessages::RequestPositions),
            Ok(62) => Ok(OutgoingMessages::RequestAccountSummary),
            Ok(63) => Ok(OutgoingMessages::CancelAccountSummary),
            Ok(64) => Ok(OutgoingMessages::CancelPositions),
            Ok(65) => Ok(OutgoingMessages::VerifyRequest),
            Ok(66) => Ok(OutgoingMessages::VerifyMessage),
            Ok(67) => Ok(OutgoingMessages::QueryDisplayGroups),
            Ok(68) => Ok(OutgoingMessages::SubscribeToGroupEvents),
            Ok(69) => Ok(OutgoingMessages::UpdateDisplayGroup),
            Ok(70) => Ok(OutgoingMessages::UnsubscribeFromGroupEvents),
            Ok(71) => Ok(OutgoingMessages::StartApi),
            Ok(72) => Ok(OutgoingMessages::VerifyAndAuthRequest),
            Ok(73) => Ok(OutgoingMessages::VerifyAndAuthMessage),
            Ok(74) => Ok(OutgoingMessages::RequestPositionsMulti),
            Ok(75) => Ok(OutgoingMessages::CancelPositionsMulti),
            Ok(76) => Ok(OutgoingMessages::RequestAccountUpdatesMulti),
            Ok(77) => Ok(OutgoingMessages::CancelAccountUpdatesMulti),
            Ok(78) => Ok(OutgoingMessages::RequestSecurityDefinitionOptionalParameters),
            Ok(79) => Ok(OutgoingMessages::RequestSoftDollarTiers),
            Ok(80) => Ok(OutgoingMessages::RequestFamilyCodes),
            Ok(81) => Ok(OutgoingMessages::RequestMatchingSymbols),
            Ok(82) => Ok(OutgoingMessages::RequestMktDepthExchanges),
            Ok(83) => Ok(OutgoingMessages::RequestSmartComponents),
            Ok(84) => Ok(OutgoingMessages::RequestNewsArticle),
            Ok(85) => Ok(OutgoingMessages::RequestNewsProviders),
            Ok(86) => Ok(OutgoingMessages::RequestHistoricalNews),
            Ok(87) => Ok(OutgoingMessages::RequestHeadTimestamp),
            Ok(88) => Ok(OutgoingMessages::RequestHistogramData),
            Ok(89) => Ok(OutgoingMessages::CancelHistogramData),
            Ok(90) => Ok(OutgoingMessages::CancelHeadTimestamp),
            Ok(91) => Ok(OutgoingMessages::RequestMarketRule),
            Ok(92) => Ok(OutgoingMessages::RequestPnL),
            Ok(93) => Ok(OutgoingMessages::CancelPnL),
            Ok(94) => Ok(OutgoingMessages::RequestPnLSingle),
            Ok(95) => Ok(OutgoingMessages::CancelPnLSingle),
            Ok(96) => Ok(OutgoingMessages::RequestHistoricalTicks),
            Ok(97) => Ok(OutgoingMessages::RequestTickByTickData),
            Ok(98) => Ok(OutgoingMessages::CancelTickByTickData),
            Ok(99) => Ok(OutgoingMessages::RequestCompletedOrders),
            Ok(100) => Ok(OutgoingMessages::RequestWshMetaData),
            Ok(101) => Ok(OutgoingMessages::CancelWshMetaData),
            Ok(102) => Ok(OutgoingMessages::RequestWshEventData),
            Ok(103) => Ok(OutgoingMessages::CancelWshEventData),
            Ok(104) => Ok(OutgoingMessages::RequestUserInfo),
            Ok(105) => Ok(OutgoingMessages::RequestCurrentTimeInMillis),
            Ok(106) => Ok(OutgoingMessages::CancelContractData),
            Ok(107) => Ok(OutgoingMessages::CancelHistoricalTicks),
            Ok(108) => Ok(OutgoingMessages::ReqConfig),
            Ok(109) => Ok(OutgoingMessages::UpdateConfig),
            Ok(n) => Err(Error::parse_field(n.to_string(), "unknown outgoing message type")),
            Err(_) => Err(Error::parse_field(s, "invalid outgoing message type")),
        }
    }
}

/// Encode the outbound message length prefix using the IB wire format.
pub(crate) fn encode_length(message: &str) -> Vec<u8> {
    encode_raw_length(message.as_bytes())
}

/// Encode a protobuf outbound message: 4-byte BE (msg_id + 200) + proto bytes.
pub(crate) fn encode_protobuf_message(msg_id: i32, proto_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + proto_bytes.len());
    buf.write_i32::<BigEndian>(msg_id + PROTOBUF_MSG_ID).unwrap();
    buf.extend_from_slice(proto_bytes);
    buf
}

/// Encode a length-prefixed raw message (4-byte BE length + data).
pub(crate) fn encode_raw_length(data: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(data.len() + 4);
    packet.write_u32::<BigEndian>(data.len() as u32).unwrap();
    packet.write_all(data).unwrap();
    packet
}

/// Builder for outbound TWS/Gateway request messages (test-only).
#[cfg(test)]
#[derive(Default, Debug, Clone)]
pub(crate) struct RequestMessage {
    pub(crate) fields: Vec<String>,
}

#[cfg(all(test, feature = "sync"))]
impl RequestMessage {
    /// Serialize all fields into the NUL-delimited wire format.
    pub(crate) fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
    }

    /// Serialize the message as a pipe-delimited string (test helper).
    pub(crate) fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }

    /// Construct a request message from a NUL-delimited string (test helper).
    pub(crate) fn from(fields: &str) -> RequestMessage {
        RequestMessage {
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
        }
    }

    /// Construct a request message from a pipe-delimited string (test helper).
    pub(crate) fn from_simple(fields: &str) -> RequestMessage {
        RequestMessage {
            fields: fields.split_terminator('|').map(|x| x.to_string()).collect(),
        }
    }
}

#[cfg(test)]
impl std::ops::Index<usize> for RequestMessage {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}

/// Minimal protobuf envelope decoding the int32 at tag 1. Covers `request_id`
/// across most TWS protobuf messages and `order_id` for `OpenOrder` /
/// `OrderStatus` / `ExecutionDetailsEnd`. Avoids the full struct decode
/// (`OpenOrder` nests `Contract` + `Order` + `OrderState` — ~30 String
/// allocations) just to read one int32; prost length-prefix-skips unknown
/// trailing tags.
#[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
struct ProtoIdEnvelope {
    #[prost(int32, optional, tag = "1")]
    pub id: Option<i32>,
}

/// Minimal envelope for `ExecutionDetails` reading only the nested
/// `execution.order_id` (tag 3 → tag 1) and `execution.exec_id` (tag 3 →
/// tag 2). Skips the `contract` sub-message at tag 2, avoiding ~20 String
/// allocations per inbound `ExecutionData`.
#[derive(Clone, PartialEq, ::prost::Message)]
struct ExecutionDetailsMinimal {
    #[prost(message, optional, tag = "3")]
    pub execution: Option<ExecutionMinimal>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct ExecutionMinimal {
    #[prost(int32, optional, tag = "1")]
    pub order_id: Option<i32>,
    #[prost(string, optional, tag = "2")]
    pub exec_id: Option<String>,
}

/// Parsed inbound message from TWS/Gateway.
///
/// Crate-internal wire envelope; not part of the public API. All fields,
/// constructors, and methods are crate-visible only.
#[derive(Clone, Default, Debug)]
pub(crate) struct ResponseMessage {
    /// Cursor index for incremental decoding.
    pub i: usize,
    /// Raw field buffer backing this message.
    pub fields: Vec<String>,
    /// Server version stored with the message for version-gated decoding.
    /// Reads disappeared with the text error accessors in PR-D1; D3 deletes
    /// the field itself once `from_protobuf` / `from_binary_text` stop
    /// plumbing it.
    #[allow(dead_code)]
    pub server_version: i32,
    /// True when the message payload is protobuf-encoded.
    /// Production reads disappeared in PR-D2 when the proto-aware accessors
    /// collapsed; remaining readers are test fixtures that mirror the wire
    /// framing. D3 deletes the field outright.
    #[allow(dead_code)]
    pub is_protobuf: bool,
    /// Raw protobuf payload bytes (everything after the 4-byte binary message ID).
    pub raw_bytes: Option<Vec<u8>>,
}

impl ResponseMessage {
    /// Build a protobuf response message from a binary message type and raw payload bytes.
    pub fn from_protobuf(message_type: i32, raw_bytes: Vec<u8>, server_version: i32) -> Self {
        Self {
            i: 0,
            fields: vec![message_type.to_string()],
            server_version,
            is_protobuf: true,
            raw_bytes: Some(raw_bytes),
        }
    }

    /// Build a text response message from a binary message ID and NUL-delimited text payload.
    /// Used when server_version >= PROTOBUF but the message ID <= 200 (text message).
    pub fn from_binary_text(msg_id: i32, text_payload: &str, server_version: i32) -> Self {
        let mut fields = vec![msg_id.to_string()];
        fields.extend(text_payload.split_terminator('\0').map(|s| s.to_string()));
        Self {
            i: 0,
            fields,
            server_version,
            is_protobuf: false,
            raw_bytes: None,
        }
    }

    /// Raw protobuf payload bytes, if this is a protobuf message.
    pub fn raw_bytes(&self) -> Option<&[u8]> {
        self.raw_bytes.as_deref()
    }

    /// Raw protobuf payload bytes for use by proto-only decoders. Text-framed
    /// arrival returns `Error::UnexpectedResponse`, which the dispatcher
    /// skip-classifies (per CLAUDE.md rule 20) rather than terminating the
    /// subscription.
    pub(crate) fn require_proto(&self) -> Result<&[u8], crate::Error> {
        self.raw_bytes().ok_or_else(|| crate::Error::unexpected_response(self))
    }

    /// Number of fields present in the message.
    #[allow(dead_code)] // test-only since text decoders are proto-only post-floor-213
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if the message informs about API shutdown.
    #[cfg_attr(not(feature = "sync"), allow(dead_code))] // sync-transport-only caller
    pub fn is_shutdown(&self) -> bool {
        self.message_type() == IncomingMessages::Shutdown
    }

    /// Return the discriminator identifying the message payload.
    pub fn message_type(&self) -> IncomingMessages {
        if self.fields.is_empty() {
            IncomingMessages::NotValid
        } else {
            let message_id = i32::from_str(&self.fields[0]).unwrap_or(-1);
            IncomingMessages::from(message_id)
        }
    }

    /// Try to extract the request id from the message.
    ///
    /// For proto-framed messages (everything past floor 213 except
    /// [`IncomingMessages::TickEFP`]), the request id lives at proto tag 1
    /// (int32) in `raw_bytes`. The text-framed branch keeps support for
    /// TickEFP, which TWS has no protobuf encoder for.
    pub fn request_id(&self) -> Option<i32> {
        let i = request_id_index(self.message_type())?;
        if let Some(raw) = self.raw_bytes() {
            let env: ProtoIdEnvelope = prost::Message::decode(raw).ok()?;
            env.id
        } else {
            self.peek_int(i).ok()
        }
    }

    /// Try to extract the order id from the message.
    ///
    /// Every `order_id`-bearing message type (`OpenOrder`, `OrderStatus`,
    /// `ExecutionData`, `ExecutionDataEnd`) is proto-only at floor 213. Three
    /// carry `order_id` at proto tag 1 (decoded via the minimal
    /// [`ProtoIdEnvelope`]); `ExecutionData` nests it under
    /// `execution.order_id` and uses [`ExecutionDetailsMinimal`] to skip the
    /// `contract` sub-message.
    pub fn order_id(&self) -> Option<i32> {
        let raw = self.raw_bytes()?;
        match self.message_type() {
            IncomingMessages::OpenOrder | IncomingMessages::OrderStatus | IncomingMessages::ExecutionDataEnd => {
                prost::Message::decode(raw).ok().and_then(|e: ProtoIdEnvelope| e.id)
            }
            IncomingMessages::ExecutionData => {
                let p: ExecutionDetailsMinimal = prost::Message::decode(raw).ok()?;
                p.execution.and_then(|e| e.order_id)
            }
            _ => None,
        }
    }

    /// Try to extract the execution id from the message.
    ///
    /// `ExecutionData` and `CommissionsReport` are proto-only at floor 213;
    /// the `exec_id` lives inside the proto payload (nested under
    /// `execution.exec_id` for `ExecutionData`; at the top level for
    /// `CommissionsReport`).
    pub fn execution_id(&self) -> Option<String> {
        let raw = self.raw_bytes()?;
        match self.message_type() {
            IncomingMessages::ExecutionData => {
                let p: ExecutionDetailsMinimal = prost::Message::decode(raw).ok()?;
                p.execution.and_then(|e| e.exec_id)
            }
            IncomingMessages::CommissionsReport => {
                let p: crate::proto::CommissionAndFeesReport = prost::Message::decode(raw).ok()?;
                p.exec_id
            }
            _ => None,
        }
    }

    /// Peek an integer field without advancing the cursor.
    ///
    /// Only callers post-floor-213 are [`Self::request_id`]'s text-fallback
    /// path for [`IncomingMessages::TickEFP`] and the legacy handshake parser.
    pub fn peek_int(&self, i: usize) -> Result<i32, Error> {
        if i >= self.fields.len() {
            return Err(Error::eof_at(i, "int"));
        }

        let field = &self.fields[i];
        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(i, field.into(), err.to_string())),
        }
    }

    /// Consume and parse the next integer field.
    pub fn next_int(&mut self) -> Result<i32, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::eof_at(self.i, "int"));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field as a string.
    pub fn next_string(&mut self) -> Result<String, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::eof_at(self.i, "string"));
        }

        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

    /// Consume and parse the next floating-point field.
    pub fn next_double(&mut self) -> Result<f64, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::eof_at(self.i, "double"));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == "0" || field == "0.0" {
            return Ok(0.0);
        }

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Build a response message from a NUL-delimited payload.
    pub fn from(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
            server_version: 0,
            is_protobuf: false,
            raw_bytes: None,
        }
    }
    #[cfg(test)]
    /// Build a response message from a pipe-delimited payload (test helper).
    pub fn from_simple(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('|').map(|x| x.to_string()).collect(),
            server_version: 0,
            is_protobuf: false,
            raw_bytes: None,
        }
    }

    /// Set the server version for version-gated decoding (builder style).
    /// Test-only post-floor-213; `parse_raw_message` always plumbs the version
    /// at construction time. D3 deletes the helper outright.
    #[cfg(test)]
    pub fn with_server_version(mut self, server_version: i32) -> Self {
        self.server_version = server_version;
        self
    }

    /// Advance the cursor past the next field.
    pub fn skip(&mut self) {
        self.i += 1;
    }

    /// Encode the message back into a NUL-delimited string.
    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
    }

    #[cfg(test)]
    /// Serialize the message into a pipe-delimited format (test helper).
    pub fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }
}

/// An error message from the TWS API.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notice {
    /// Error code reported by TWS.
    pub code: i32,
    /// Human-readable error message text.
    pub message: String,
    /// Timestamp when the error occurred.
    /// Only present for server versions >= ERROR_TIME (194).
    pub error_time: Option<OffsetDateTime>,
    /// Advanced order-reject JSON payload, present on hard order-rejection
    /// notices for server versions >= ADVANCED_ORDER_REJECT. Empty otherwise.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub advanced_order_reject_json: String,
}

/// Error code indicating an order was cancelled (confirmation, not an error).
pub const ORDER_CANCELLED_CODE: i32 = 202;

/// Range of error codes that are considered warnings (2100-2169).
pub const WARNING_CODE_RANGE: std::ops::RangeInclusive<i32> = 2100..=2169;

/// System message codes indicating connectivity status.
/// - 1100: Connectivity lost
/// - 1101: Connectivity restored, market data lost (resubscribe needed)
/// - 1102: Connectivity restored, market data maintained
/// - 1300: Socket port reset during active connection
pub const SYSTEM_MESSAGE_CODES: [i32; 4] = [1100, 1101, 1102, 1300];

/// Range of error codes that represent order rejections from TWS (200-399).
///
/// Includes parameter validation, contract-not-found, margin and risk-check
/// rejections. Note: [`ORDER_CANCELLED_CODE`] (202) is numerically inside this
/// range but is a *confirmation*, not a rejection; see [`Notice::category`]
/// for partition semantics.
pub const ORDER_REJECTION_CODE_RANGE: std::ops::RangeInclusive<i32> = 200..=399;

/// Synthesized notice code emitted when a handshake-time frame's
/// [`IncomingMessages`] kind has no typed `StartupMessage` variant. Negative
/// (TWS uses 0+); the only other sentinel in this range is `-2` for the
/// gateway-initiated shutdown signal. See [`Notice::is_handshake_synthetic`].
pub const HANDSHAKE_UNKNOWN_FRAME_CODE: i32 = -3;

/// Synthesized notice code emitted when a typed handshake decoder fails for
/// a known [`IncomingMessages`] kind (`OpenOrder`, `OrderStatus`,
/// `AccountValue`/`PortfolioValue`/`AccountUpdateTime`/`AccountDownloadEnd`,
/// `ExecutionData`, `CommissionsReport`, `CompletedOrder`). Distinct from
/// [`HANDSHAKE_UNKNOWN_FRAME_CODE`] so consumers can separate TWS schema
/// drift from rust-ibapi decoder bugs. See
/// [`Notice::is_handshake_synthetic`].
pub const HANDSHAKE_DECODE_FAILURE_CODE: i32 = -4;

/// Typed classification of a [`Notice`] by TWS error-code range.
///
/// Returned by [`Notice::category`]. Forms a disjoint partition over all
/// possible codes; when ranges overlap on the wire (e.g. code 202 is both a
/// cancellation and inside the order-rejection range 200-399), the classifier
/// resolves overlap by **precedence**:
///
/// 1. [`Cancellation`](Self::Cancellation) — exact code 202.
/// 2. [`Warning`](Self::Warning) — 2100..=2169.
/// 3. [`SystemMessage`](Self::SystemMessage) — 1100, 1101, 1102, 1300.
/// 4. [`OrderRejection`](Self::OrderRejection) — 200..=399, excluding 202 by precedence.
/// 5. [`Error`](Self::Error) — everything else.
///
/// Marked `#[non_exhaustive]` so IBKR can introduce new code ranges without a
/// breaking release.
///
/// # Examples
///
/// ```no_run
/// use ibapi::{Notice, NoticeCategory};
/// # let notice: Notice = unimplemented!();
/// match notice.category() {
///     NoticeCategory::OrderRejection => eprintln!("rejected: {}", notice),
///     NoticeCategory::Warning        => eprintln!("warn: {}",     notice),
///     NoticeCategory::Error          => eprintln!("error: {}",    notice),
///     _ => {}
/// }
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeCategory {
    /// Order cancellation confirmation (exact code 202).
    Cancellation,
    /// Informational warning (codes 2100..=2169).
    Warning,
    /// Connectivity / system status (codes 1100, 1101, 1102, 1300).
    SystemMessage,
    /// Order rejection (codes 200..=399).
    OrderRejection,
    /// Any other error code.
    Error,
}

impl From<&ResponseMessage> for Notice {
    /// Build a Notice from a protobuf Error frame; at floor 213 every Error
    /// payload arrives proto-encoded. Returns `Notice::from(DecodedError::default())`
    /// (empty / code 0) if the proto bytes are absent or undecodable.
    fn from(message: &ResponseMessage) -> Notice {
        let payload = message
            .raw_bytes()
            .and_then(crate::transport::routing::decode_error_envelope)
            .unwrap_or_default();
        Notice::from(payload)
    }
}

impl From<&mut ResponseMessage> for Notice {
    /// Convenience for decoder call sites that hold `&mut ResponseMessage`.
    /// Trait dispatch doesn't auto-coerce `&mut T → &T`, so this shim avoids
    /// `Notice::from(&*message)` boilerplate.
    fn from(message: &mut ResponseMessage) -> Notice {
        Notice::from(&*message)
    }
}

impl Notice {
    /// Build a client-synthesized notice with no wire timestamp and no
    /// advanced-order-reject JSON. Used by handshake-time observability
    /// shims (see [`HANDSHAKE_UNKNOWN_FRAME_CODE`] /
    /// [`HANDSHAKE_DECODE_FAILURE_CODE`]).
    pub(crate) fn synthesized(code: i32, message: String) -> Notice {
        Notice {
            code,
            message,
            error_time: None,
            advanced_order_reject_json: String::new(),
        }
    }

    /// Returns `true` if this notice indicates an order was cancelled (code 202).
    ///
    /// Code 202 is sent by TWS to confirm an order cancellation. This is an
    /// informational message, not an error.
    pub fn is_cancellation(&self) -> bool {
        self.code == ORDER_CANCELLED_CODE
    }

    /// Returns `true` if this is a warning message (codes 2100-2169).
    pub fn is_warning(&self) -> bool {
        WARNING_CODE_RANGE.contains(&self.code)
    }

    /// Returns `true` if this is a system/connectivity message (codes 1100-1102, 1300).
    ///
    /// System messages indicate connectivity status changes:
    /// - 1100: Connectivity between IB and TWS lost
    /// - 1101: Connectivity restored, market data lost (resubscribe needed)
    /// - 1102: Connectivity restored, market data maintained
    /// - 1300: Socket port reset during active connection
    pub fn is_system_message(&self) -> bool {
        SYSTEM_MESSAGE_CODES.contains(&self.code)
    }

    /// Returns `true` if this is an informational notice (not an error).
    ///
    /// Informational notices include cancellation confirmations, warnings,
    /// and system/connectivity messages.
    pub fn is_informational(&self) -> bool {
        self.is_cancellation() || self.is_warning() || self.is_system_message()
    }

    /// Returns `true` if this is an error requiring attention.
    ///
    /// Returns `false` for informational messages like cancellation confirmations,
    /// warnings, and system messages.
    pub fn is_error(&self) -> bool {
        !self.is_informational()
    }

    /// Returns `true` if this notice falls in the order-rejection range (200-399).
    ///
    /// Code 202 (cancellation confirmation) is numerically inside this range; this
    /// predicate returns `true` for it. For a disjoint partition that routes 202
    /// to [`NoticeCategory::Cancellation`] instead, use [`Notice::category`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_order_rejection() {
    ///     eprintln!("rejection: {}", notice);
    /// }
    /// ```
    pub fn is_order_rejection(&self) -> bool {
        ORDER_REJECTION_CODE_RANGE.contains(&self.code)
    }

    /// Returns `true` if this notice was synthesized client-side during the
    /// connection handshake — i.e. carries either
    /// [`HANDSHAKE_UNKNOWN_FRAME_CODE`] (an `IncomingMessages` kind with no
    /// typed `StartupMessage` variant) or [`HANDSHAKE_DECODE_FAILURE_CODE`]
    /// (a typed decoder failed on a known kind).
    ///
    /// Subscribers to [`Client::notice_stream`](crate::Client::notice_stream)
    /// can use this to log + investigate without conflating with TWS-emitted
    /// notices.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Notice;
    /// # let notice: Notice = unimplemented!();
    /// if notice.is_handshake_synthetic() {
    ///     eprintln!("handshake observability: code={} {}", notice.code, notice);
    /// }
    /// ```
    pub fn is_handshake_synthetic(&self) -> bool {
        self.code == HANDSHAKE_UNKNOWN_FRAME_CODE || self.code == HANDSHAKE_DECODE_FAILURE_CODE
    }

    /// Classify this notice into a disjoint [`NoticeCategory`].
    ///
    /// See [`NoticeCategory`] for the precedence chain.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::{Notice, NoticeCategory};
    /// # let notice: Notice = unimplemented!();
    /// let level = match notice.category() {
    ///     NoticeCategory::Cancellation
    ///     | NoticeCategory::Warning
    ///     | NoticeCategory::SystemMessage => "info",
    ///     NoticeCategory::OrderRejection | NoticeCategory::Error => "error",
    ///     _ => "unknown",
    /// };
    /// # let _ = level;
    /// ```
    pub fn category(&self) -> NoticeCategory {
        if self.is_cancellation() {
            NoticeCategory::Cancellation
        } else if self.is_warning() {
            NoticeCategory::Warning
        } else if self.is_system_message() {
            NoticeCategory::SystemMessage
        } else if self.is_order_rejection() {
            NoticeCategory::OrderRejection
        } else {
            NoticeCategory::Error
        }
    }
}

impl Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl From<crate::transport::routing::DecodedError> for Notice {
    /// Build a Notice from a dispatcher-decoded error payload, moving the
    /// `error_message` and `advanced_order_reject_json` strings and converting
    /// `error_time` (millis-since-epoch) to `OffsetDateTime`.
    fn from(payload: crate::transport::routing::DecodedError) -> Notice {
        let error_time = payload
            .error_time
            .and_then(|millis| OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).ok());
        Notice {
            code: payload.error_code,
            message: payload.error_message,
            error_time,
            advanced_order_reject_json: payload.advanced_order_reject_json,
        }
    }
}
