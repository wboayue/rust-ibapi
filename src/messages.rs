//! Message encoding, decoding, and routing for TWS API communication.
//!
//! This module handles the low-level message protocol between the client and TWS,
//! including request/response message formatting, field encoding/decoding,
//! and message type definitions.

use std::fmt::Display;
use std::io::Write;
use std::ops::Index;
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

const INFINITY_STR: &str = "Infinity";
const UNSET_DOUBLE: &str = "1.7976931348623157E308";
const UNSET_INTEGER: &str = "2147483647";
const UNSET_LONG: &str = "9223372036854775807";

// Index of message text in the response message
pub(crate) const MESSAGE_INDEX: usize = 4;
// Index of message code in the response message
pub(crate) const CODE_INDEX: usize = 3;

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
            _ => IncomingMessages::NotValid,
        }
    }
}

impl FromStr for IncomingMessages {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(n) => Ok(IncomingMessages::from(n)),
            Err(_) => Err(Error::Simple(format!("Invalid incoming message type: {}", s))),
        }
    }
}

/// Return the message field index containing the order id, if present.
pub fn order_id_index(kind: IncomingMessages) -> Option<usize> {
    match kind {
        IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => Some(1),
        IncomingMessages::ExecutionData | IncomingMessages::ExecutionDataEnd => Some(2),
        _ => None,
    }
}

/// Return the message field index containing the request id, if present.
pub fn request_id_index(kind: IncomingMessages) -> Option<usize> {
    match kind {
        IncomingMessages::AccountSummary => Some(2),
        IncomingMessages::AccountSummaryEnd => Some(2),
        IncomingMessages::AccountUpdateMulti => Some(2),
        IncomingMessages::AccountUpdateMultiEnd => Some(2),
        IncomingMessages::ContractData => Some(1),
        IncomingMessages::ContractDataEnd => Some(2),
        IncomingMessages::Error => Some(2),
        IncomingMessages::ExecutionData => Some(1),
        IncomingMessages::ExecutionDataEnd => Some(2),
        IncomingMessages::HeadTimestamp => Some(1),
        IncomingMessages::HistogramData => Some(1),
        IncomingMessages::HistoricalData => Some(1),
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
    /// Request streaming market data (`reqMktData`).
    RequestMarketData = 1,
    /// Cancel streaming market data (`cancelMktData`).
    CancelMarketData = 2,
    /// Submit a new order (`placeOrder`).
    PlaceOrder = 3,
    /// Cancel an existing order (`cancelOrder`).
    CancelOrder = 4,
    /// Request the current open orders (`reqOpenOrders`).
    RequestOpenOrders = 5,
    /// Request account value updates (`reqAccountUpdates`).
    RequestAccountData = 6,
    /// Request execution reports (`reqExecutions`).
    RequestExecutions = 7,
    /// Request a block of valid order ids (`reqIds`).
    RequestIds = 8,
    /// Request contract details (`reqContractDetails`).
    RequestContractData = 9,
    /// Request level-two market depth (`reqMktDepth`).
    RequestMarketDepth = 10,
    /// Cancel level-two market depth (`cancelMktDepth`).
    CancelMarketDepth = 11,
    /// Subscribe to news bulletins (`reqNewsBulletins`).
    RequestNewsBulletins = 12,
    /// Cancel news bulletin subscription (`cancelNewsBulletins`).
    CancelNewsBulletin = 13,
    /// Change the server log level (`setServerLogLevel`).
    ChangeServerLog = 14,
    /// Request auto-open orders (`reqAutoOpenOrders`).
    RequestAutoOpenOrders = 15,
    /// Request all open orders (`reqAllOpenOrders`).
    RequestAllOpenOrders = 16,
    /// Request managed accounts list (`reqManagedAccts`).
    RequestManagedAccounts = 17,
    /// Request financial advisor configuration (`requestFA`).
    RequestFA = 18,
    /// Replace financial advisor configuration (`replaceFA`).
    ReplaceFA = 19,
    /// Request historical bar data (`reqHistoricalData`).
    RequestHistoricalData = 20,
    /// Exercise an option contract (`exerciseOptions`).
    ExerciseOptions = 21,
    /// Subscribe to a market scanner (`reqScannerSubscription`).
    RequestScannerSubscription = 22,
    /// Cancel a market scanner subscription (`cancelScannerSubscription`).
    CancelScannerSubscription = 23,
    /// Request scanner parameter definitions (`reqScannerParameters`).
    RequestScannerParameters = 24,
    /// Cancel an in-flight historical data request (`cancelHistoricalData`).
    CancelHistoricalData = 25,
    /// Request the current TWS/Gateway time (`reqCurrentTime`).
    RequestCurrentTime = 49,
    /// Request real-time bars (`reqRealTimeBars`).
    RequestRealTimeBars = 50,
    /// Cancel real-time bars (`cancelRealTimeBars`).
    CancelRealTimeBars = 51,
    /// Request fundamental data (`reqFundamentalData`).
    RequestFundamentalData = 52,
    /// Cancel fundamental data (`cancelFundamentalData`).
    CancelFundamentalData = 53,
    /// Request implied volatility calculation (`calculateImpliedVolatility`).
    ReqCalcImpliedVolat = 54,
    /// Request option price calculation (`calculateOptionPrice`).
    ReqCalcOptionPrice = 55,
    /// Cancel implied volatility calculation (`cancelImpliedVolatility`).
    CancelImpliedVolatility = 56,
    /// Cancel option price calculation (`cancelCalculateOptionPrice`).
    CancelOptionPrice = 57,
    /// Issue a global cancel request (`reqGlobalCancel`).
    RequestGlobalCancel = 58,
    /// Change the active market data type (`reqMarketDataType`).
    RequestMarketDataType = 59,
    /// Subscribe to position updates (`reqPositions`).
    RequestPositions = 61,
    /// Subscribe to account summary (`reqAccountSummary`).
    RequestAccountSummary = 62,
    /// Cancel account summary subscription (`cancelAccountSummary`).
    CancelAccountSummary = 63,
    /// Cancel position subscription (`cancelPositions`).
    CancelPositions = 64,
    /// Begin API verification handshake (`verifyRequest`).
    VerifyRequest = 65,
    /// Respond to verification handshake (`verifyMessage`).
    VerifyMessage = 66,
    /// Query display groups (`queryDisplayGroups`).
    QueryDisplayGroups = 67,
    /// Subscribe to display group events (`subscribeToGroupEvents`).
    SubscribeToGroupEvents = 68,
    /// Update a display group subscription (`updateDisplayGroup`).
    UpdateDisplayGroup = 69,
    /// Unsubscribe from display group events (`unsubscribeFromGroupEvents`).
    UnsubscribeFromGroupEvents = 70,
    /// Start the API session (`startApi`).
    StartApi = 71,
    /// Verification handshake with auth (`verifyAndAuthRequest`).
    VerifyAndAuthRequest = 72,
    /// Verification message with auth (`verifyAndAuthMessage`).
    VerifyAndAuthMessage = 73,
    /// Request multi-account/model positions (`reqPositionsMulti`).
    RequestPositionsMulti = 74,
    /// Cancel multi-account/model positions (`cancelPositionsMulti`).
    CancelPositionsMulti = 75,
    /// Request multi-account/model updates (`reqAccountUpdatesMulti`).
    RequestAccountUpdatesMulti = 76,
    /// Cancel multi-account/model updates (`cancelAccountUpdatesMulti`).
    CancelAccountUpdatesMulti = 77,
    /// Request optional option security parameters (`reqSecDefOptParams`).
    RequestSecurityDefinitionOptionalParameters = 78,
    /// Request soft-dollar tier definitions (`reqSoftDollarTiers`).
    RequestSoftDollarTiers = 79,
    /// Request family codes (`reqFamilyCodes`).
    RequestFamilyCodes = 80,
    /// Request matching symbols (`reqMatchingSymbols`).
    RequestMatchingSymbols = 81,
    /// Request exchanges that support depth (`reqMktDepthExchanges`).
    RequestMktDepthExchanges = 82,
    /// Request smart routing component map (`reqSmartComponents`).
    RequestSmartComponents = 83,
    /// Request detailed news article (`reqNewsArticle`).
    RequestNewsArticle = 84,
    /// Request available news providers (`reqNewsProviders`).
    RequestNewsProviders = 85,
    /// Request historical news headlines (`reqHistoricalNews`).
    RequestHistoricalNews = 86,
    /// Request earliest timestamp for historical data (`reqHeadTimestamp`).
    RequestHeadTimestamp = 87,
    /// Request histogram snapshot (`reqHistogramData`).
    RequestHistogramData = 88,
    /// Cancel histogram snapshot (`cancelHistogramData`).
    CancelHistogramData = 89,
    /// Cancel head timestamp request (`cancelHeadTimestamp`).
    CancelHeadTimestamp = 90,
    /// Request market rule definition (`reqMarketRule`).
    RequestMarketRule = 91,
    /// Request account-wide PnL stream (`reqPnL`).
    RequestPnL = 92,
    /// Cancel account-wide PnL stream (`cancelPnL`).
    CancelPnL = 93,
    /// Request single-position PnL stream (`reqPnLSingle`).
    RequestPnLSingle = 94,
    /// Cancel single-position PnL stream (`cancelPnLSingle`).
    CancelPnLSingle = 95,
    /// Request historical tick data (`reqHistoricalTicks`).
    RequestHistoricalTicks = 96,
    /// Request tick-by-tick data (`reqTickByTickData`).
    RequestTickByTickData = 97,
    /// Cancel tick-by-tick data (`cancelTickByTickData`).
    CancelTickByTickData = 98,
    /// Request completed order history (`reqCompletedOrders`).
    RequestCompletedOrders = 99,
    /// Request Wall Street Horizon metadata (`reqWshMetaData`).
    RequestWshMetaData = 100,
    /// Cancel Wall Street Horizon metadata (`cancelWshMetaData`).
    CancelWshMetaData = 101,
    /// Request Wall Street Horizon event data (`reqWshEventData`).
    RequestWshEventData = 102,
    /// Cancel Wall Street Horizon event data (`cancelWshEventData`).
    CancelWshEventData = 103,
    /// Request user information (`reqUserInfo`).
    RequestUserInfo = 104,
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
            Ok(n) => Err(Error::Simple(format!("Unknown outgoing message type: {}", n))),
            Err(_) => Err(Error::Simple(format!("Invalid outgoing message type: {}", s))),
        }
    }
}

/// Encode the outbound message length prefix using the IB wire format.
pub fn encode_length(message: &str) -> Vec<u8> {
    let data = message.as_bytes();

    let mut packet: Vec<u8> = Vec::with_capacity(data.len() + 4);

    packet.write_u32::<BigEndian>(data.len() as u32).unwrap();
    packet.write_all(data).unwrap();
    packet
}

/// Builder for outbound TWS/Gateway request messages.
#[derive(Default, Debug, Clone)]
pub struct RequestMessage {
    pub(crate) fields: Vec<String>,
}

impl RequestMessage {
    /// Create a new empty request message.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_field<T: ToField>(&mut self, val: &T) -> &RequestMessage {
        let field = val.to_field();
        self.fields.push(field);
        self
    }

    /// Serialize all fields into the NUL-delimited wire format.
    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.fields.len()
    }

    #[cfg(test)]
    /// Serialize the message as a pipe-delimited string (test helper).
    pub(crate) fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }
    #[cfg(test)]
    /// Construct a request message from a NUL-delimited string (test helper).
    pub fn from(fields: &str) -> RequestMessage {
        RequestMessage {
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
        }
    }
    #[cfg(test)]
    /// Construct a request message from a pipe-delimited string (test helper).
    pub fn from_simple(fields: &str) -> RequestMessage {
        RequestMessage {
            fields: fields.split_terminator('|').map(|x| x.to_string()).collect(),
        }
    }
}

impl Index<usize> for RequestMessage {
    type Output = String;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}

/// Parsed inbound message from TWS/Gateway.
#[derive(Clone, Default, Debug)]
pub struct ResponseMessage {
    /// Cursor index for incremental decoding.
    pub i: usize,
    /// Raw field buffer backing this message.
    pub fields: Vec<String>,
}

impl ResponseMessage {
    /// Number of fields present in the message.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if the message contains no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns `true` if the message informs about API shutdown.
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
    pub fn request_id(&self) -> Option<i32> {
        if let Some(i) = request_id_index(self.message_type()) {
            if let Ok(request_id) = self.peek_int(i) {
                return Some(request_id);
            }
        }
        None
    }

    /// Try to extract the order id from the message.
    pub fn order_id(&self) -> Option<i32> {
        if let Some(i) = order_id_index(self.message_type()) {
            if let Ok(order_id) = self.peek_int(i) {
                return Some(order_id);
            }
        }
        None
    }

    /// Try to extract the execution id from the message.
    pub fn execution_id(&self) -> Option<String> {
        match self.message_type() {
            IncomingMessages::ExecutionData => Some(self.peek_string(14)),
            IncomingMessages::CommissionsReport => Some(self.peek_string(2)),
            _ => None,
        }
    }

    /// Peek an integer field without advancing the cursor.
    pub fn peek_int(&self, i: usize) -> Result<i32, Error> {
        if i >= self.fields.len() {
            return Err(Error::Simple("expected int and found end of message".into()));
        }

        let field = &self.fields[i];
        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(i, field.into(), err.to_string())),
        }
    }

    /// Peek a string field without advancing the cursor.
    pub fn peek_string(&self, i: usize) -> String {
        self.fields[i].to_owned()
    }

    /// Consume and parse the next integer field.
    pub fn next_int(&mut self) -> Result<i32, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected int and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field returning `None` when unset.
    pub fn next_optional_int(&mut self) -> Result<Option<i32>, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected optional int and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_INTEGER {
            return Ok(None);
        }

        match field.parse::<i32>() {
            Ok(val) => Ok(Some(val)),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field as a boolean (`"0"` or `"1"`).
    pub fn next_bool(&mut self) -> Result<bool, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected bool and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        Ok(field == "1")
    }

    /// Consume and parse the next i64 field.
    pub fn next_long(&mut self) -> Result<i64, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected long and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field as an optional i64.
    pub fn next_optional_long(&mut self) -> Result<Option<i64>, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected optional long and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_LONG {
            return Ok(None);
        }

        match field.parse::<i64>() {
            Ok(val) => Ok(Some(val)),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field and parse it as a UTC timestamp.
    pub fn next_date_time(&mut self) -> Result<OffsetDateTime, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected datetime and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() {
            return Err(Error::Simple("expected timestamp and found empty string".into()));
        }

        // from_unix_timestamp
        let timestamp: i64 = field.parse()?;
        match OffsetDateTime::from_unix_timestamp(timestamp) {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Consume the next field as a string.
    pub fn next_string(&mut self) -> Result<String, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected string and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

    /// Consume and parse the next floating-point field.
    pub fn next_double(&mut self) -> Result<f64, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected double and found end of message".into()));
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

    /// Consume the next field as an optional floating-point value.
    pub fn next_optional_double(&mut self) -> Result<Option<f64>, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected optional double and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        if field.is_empty() || field == UNSET_DOUBLE {
            return Ok(None);
        }

        if field == INFINITY_STR {
            return Ok(Some(f64::INFINITY));
        }

        match field.parse() {
            Ok(val) => Ok(Some(val)),
            Err(err) => Err(Error::Parse(self.i, field.into(), err.to_string())),
        }
    }

    /// Build a response message from a NUL-delimited payload.
    pub fn from(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
        }
    }
    #[cfg(test)]
    /// Build a response message from a pipe-delimited payload (test helper).
    pub fn from_simple(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('|').map(|x| x.to_string()).collect(),
        }
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notice {
    /// Error code reported by TWS.
    pub code: i32,
    /// Human-readable error message text.
    pub message: String,
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

impl Notice {
    #[allow(private_interfaces)]
    /// Construct a notice from a response message.
    pub fn from(message: &ResponseMessage) -> Notice {
        let code = message.peek_int(CODE_INDEX).unwrap_or(-1);
        let message = message.peek_string(MESSAGE_INDEX);
        Notice { code, message }
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
}

impl Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}
