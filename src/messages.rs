//! Message encoding, decoding, and routing for TWS API communication.
//!
//! This module handles the low-level message protocol between the client and TWS,
//! including request/response message formatting, field encoding/decoding,
//! and message type definitions.

use std::io::Write;
use std::str::{self, FromStr};

use byteorder::{BigEndian, WriteBytesExt};

use crate::{Error, ToField};

mod notice;
pub(crate) mod shared_channel_configuration;
#[cfg(test)]
mod tests;

pub use notice::*;

/// Offset added to outbound protobuf message IDs. Inbound IDs > this value are protobuf.
pub(crate) const PROTOBUF_MSG_ID: i32 = 200;

/// Width of the big-endian message id that opens every frame body:
/// `[4-byte BE msg_id][payload]`.
pub(crate) const MESSAGE_ID_LEN: usize = 4;

/// Messages emitted by TWS/Gateway over the market data socket.
#[derive(Debug, Default, PartialEq, Eq, Hash, Copy, Clone)]
pub enum IncomingMessages {
    /// Gateway initiated shutdown.
    Shutdown = -2,
    /// Unknown or unsupported message id.
    #[default]
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
    /// Fundamental data response. IBKR removed the fundamental-data feature in
    /// TWS 10.47; no decoder claims this id, but the variant is retained so
    /// `From<i32>` maps id 51 to a known-but-unclaimed message type (see `TickEFP`).
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

/// Allow-list of incoming message types that route by `request_id`.
///
/// `ResponseMessage::request_id()` decodes the proto envelope only when the
/// message type appears here; this prevents misrouting messages that happen
/// to carry an unrelated `int32 @ tag 1` (e.g. `MarketRule.market_rule_id`,
/// `OrderBound.perm_id`). Derived from [`text_request_id_field`] — every
/// known request-scoped message has a text-frame fallback index, so a single
/// table is the source of truth.
pub(crate) fn routes_by_request_id(kind: IncomingMessages) -> bool {
    text_request_id_field(kind).is_some()
}

/// Text-format field index carrying the request id, for messages parsed
/// from pipe-delimited text framing. Doubles as the routing allow-list (see
/// [`routes_by_request_id`]).
///
/// At floor 213 no production message arrives text-framed — proto-framed
/// messages decode the request id via the envelope in `raw_bytes` instead.
/// The table remains because tests still construct text-framed
/// [`ResponseMessage`] fixtures for many proto-only message types.
/// `ExecutionData{,End}` are present because the order router falls back to
/// the request_id channel after a missed order_id lookup. `OpenOrder` routes
/// as an order message before request-id routing is considered, so its entry
/// here never routes it.
pub(crate) fn text_request_id_field(kind: IncomingMessages) -> Option<usize> {
    match kind {
        IncomingMessages::AccountSummary
        | IncomingMessages::AccountSummaryEnd
        | IncomingMessages::AccountUpdateMulti
        | IncomingMessages::AccountUpdateMultiEnd
        | IncomingMessages::ContractDataEnd
        | IncomingMessages::DisplayGroupList
        | IncomingMessages::DisplayGroupUpdated
        | IncomingMessages::ExecutionDataEnd
        | IncomingMessages::MarketDataType
        | IncomingMessages::MarketDepth
        | IncomingMessages::MarketDepthL2
        | IncomingMessages::PositionMulti
        | IncomingMessages::PositionMultiEnd
        | IncomingMessages::RealTimeBars
        | IncomingMessages::ScannerData
        | IncomingMessages::TickGeneric
        | IncomingMessages::TickPrice
        | IncomingMessages::TickSize
        | IncomingMessages::TickSnapshotEnd
        | IncomingMessages::TickString => Some(2),

        IncomingMessages::ConfigResponse
        | IncomingMessages::UpdateConfigResponse
        | IncomingMessages::BondContractData
        | IncomingMessages::ContractData
        | IncomingMessages::ExecutionData
        | IncomingMessages::HeadTimestamp
        | IncomingMessages::HistogramData
        | IncomingMessages::HistoricalData
        | IncomingMessages::HistoricalDataEnd
        | IncomingMessages::HistoricalDataUpdate
        | IncomingMessages::HistoricalNews
        | IncomingMessages::HistoricalNewsEnd
        | IncomingMessages::HistoricalSchedule
        | IncomingMessages::HistoricalTick
        | IncomingMessages::HistoricalTickBidAsk
        | IncomingMessages::HistoricalTickLast
        | IncomingMessages::NewsArticle
        | IncomingMessages::OpenOrder
        | IncomingMessages::PnL
        | IncomingMessages::PnLSingle
        | IncomingMessages::ReplaceFAEnd
        | IncomingMessages::SecurityDefinitionOptionParameter
        | IncomingMessages::SecurityDefinitionOptionParameterEnd
        | IncomingMessages::SmartComponents
        | IncomingMessages::SoftDollarTier
        | IncomingMessages::SymbolSamples
        | IncomingMessages::TickByTick
        | IncomingMessages::TickNews
        | IncomingMessages::TickOptionComputation
        | IncomingMessages::TickReqParams
        | IncomingMessages::UserInfo
        | IncomingMessages::WshEventData
        | IncomingMessages::WshMetaData => Some(1),

        _ => None,
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

impl TryFrom<i32> for OutgoingMessages {
    type Error = Error;

    fn try_from(id: i32) -> Result<Self, Self::Error> {
        match id {
            1 => Ok(OutgoingMessages::RequestMarketData),
            2 => Ok(OutgoingMessages::CancelMarketData),
            3 => Ok(OutgoingMessages::PlaceOrder),
            4 => Ok(OutgoingMessages::CancelOrder),
            5 => Ok(OutgoingMessages::RequestOpenOrders),
            6 => Ok(OutgoingMessages::RequestAccountData),
            7 => Ok(OutgoingMessages::RequestExecutions),
            8 => Ok(OutgoingMessages::RequestIds),
            9 => Ok(OutgoingMessages::RequestContractData),
            10 => Ok(OutgoingMessages::RequestMarketDepth),
            11 => Ok(OutgoingMessages::CancelMarketDepth),
            12 => Ok(OutgoingMessages::RequestNewsBulletins),
            13 => Ok(OutgoingMessages::CancelNewsBulletin),
            14 => Ok(OutgoingMessages::ChangeServerLog),
            15 => Ok(OutgoingMessages::RequestAutoOpenOrders),
            16 => Ok(OutgoingMessages::RequestAllOpenOrders),
            17 => Ok(OutgoingMessages::RequestManagedAccounts),
            18 => Ok(OutgoingMessages::RequestFA),
            19 => Ok(OutgoingMessages::ReplaceFA),
            20 => Ok(OutgoingMessages::RequestHistoricalData),
            21 => Ok(OutgoingMessages::ExerciseOptions),
            22 => Ok(OutgoingMessages::RequestScannerSubscription),
            23 => Ok(OutgoingMessages::CancelScannerSubscription),
            24 => Ok(OutgoingMessages::RequestScannerParameters),
            25 => Ok(OutgoingMessages::CancelHistoricalData),
            49 => Ok(OutgoingMessages::RequestCurrentTime),
            50 => Ok(OutgoingMessages::RequestRealTimeBars),
            51 => Ok(OutgoingMessages::CancelRealTimeBars),
            52 => Ok(OutgoingMessages::RequestFundamentalData),
            53 => Ok(OutgoingMessages::CancelFundamentalData),
            54 => Ok(OutgoingMessages::ReqCalcImpliedVolat),
            55 => Ok(OutgoingMessages::ReqCalcOptionPrice),
            56 => Ok(OutgoingMessages::CancelImpliedVolatility),
            57 => Ok(OutgoingMessages::CancelOptionPrice),
            58 => Ok(OutgoingMessages::RequestGlobalCancel),
            59 => Ok(OutgoingMessages::RequestMarketDataType),
            61 => Ok(OutgoingMessages::RequestPositions),
            62 => Ok(OutgoingMessages::RequestAccountSummary),
            63 => Ok(OutgoingMessages::CancelAccountSummary),
            64 => Ok(OutgoingMessages::CancelPositions),
            65 => Ok(OutgoingMessages::VerifyRequest),
            66 => Ok(OutgoingMessages::VerifyMessage),
            67 => Ok(OutgoingMessages::QueryDisplayGroups),
            68 => Ok(OutgoingMessages::SubscribeToGroupEvents),
            69 => Ok(OutgoingMessages::UpdateDisplayGroup),
            70 => Ok(OutgoingMessages::UnsubscribeFromGroupEvents),
            71 => Ok(OutgoingMessages::StartApi),
            72 => Ok(OutgoingMessages::VerifyAndAuthRequest),
            73 => Ok(OutgoingMessages::VerifyAndAuthMessage),
            74 => Ok(OutgoingMessages::RequestPositionsMulti),
            75 => Ok(OutgoingMessages::CancelPositionsMulti),
            76 => Ok(OutgoingMessages::RequestAccountUpdatesMulti),
            77 => Ok(OutgoingMessages::CancelAccountUpdatesMulti),
            78 => Ok(OutgoingMessages::RequestSecurityDefinitionOptionalParameters),
            79 => Ok(OutgoingMessages::RequestSoftDollarTiers),
            80 => Ok(OutgoingMessages::RequestFamilyCodes),
            81 => Ok(OutgoingMessages::RequestMatchingSymbols),
            82 => Ok(OutgoingMessages::RequestMktDepthExchanges),
            83 => Ok(OutgoingMessages::RequestSmartComponents),
            84 => Ok(OutgoingMessages::RequestNewsArticle),
            85 => Ok(OutgoingMessages::RequestNewsProviders),
            86 => Ok(OutgoingMessages::RequestHistoricalNews),
            87 => Ok(OutgoingMessages::RequestHeadTimestamp),
            88 => Ok(OutgoingMessages::RequestHistogramData),
            89 => Ok(OutgoingMessages::CancelHistogramData),
            90 => Ok(OutgoingMessages::CancelHeadTimestamp),
            91 => Ok(OutgoingMessages::RequestMarketRule),
            92 => Ok(OutgoingMessages::RequestPnL),
            93 => Ok(OutgoingMessages::CancelPnL),
            94 => Ok(OutgoingMessages::RequestPnLSingle),
            95 => Ok(OutgoingMessages::CancelPnLSingle),
            96 => Ok(OutgoingMessages::RequestHistoricalTicks),
            97 => Ok(OutgoingMessages::RequestTickByTickData),
            98 => Ok(OutgoingMessages::CancelTickByTickData),
            99 => Ok(OutgoingMessages::RequestCompletedOrders),
            100 => Ok(OutgoingMessages::RequestWshMetaData),
            101 => Ok(OutgoingMessages::CancelWshMetaData),
            102 => Ok(OutgoingMessages::RequestWshEventData),
            103 => Ok(OutgoingMessages::CancelWshEventData),
            104 => Ok(OutgoingMessages::RequestUserInfo),
            105 => Ok(OutgoingMessages::RequestCurrentTimeInMillis),
            106 => Ok(OutgoingMessages::CancelContractData),
            107 => Ok(OutgoingMessages::CancelHistoricalTicks),
            108 => Ok(OutgoingMessages::ReqConfig),
            109 => Ok(OutgoingMessages::UpdateConfig),
            n => Err(Error::parse_field(n.to_string(), "unknown outgoing message type")),
        }
    }
}

impl FromStr for OutgoingMessages {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = s.parse::<i32>().map_err(|_| Error::parse_field(s, "invalid outgoing message type"))?;
        OutgoingMessages::try_from(id)
    }
}

impl OutgoingMessages {
    /// Whether this message stops something already running: a `Cancel*`,
    /// the global order cancel, or leaving display-group events. The outbound
    /// rate limiter sends these without waiting. Exhaustive so a new variant
    /// forces a decision.
    ///
    /// Cancels that reuse a request id with a flag, such as
    /// `RequestAccountData` with `subscribe = false`, are not recognized and
    /// wait like requests: slower, never over the limit.
    pub(crate) fn is_cancel(self) -> bool {
        match self {
            OutgoingMessages::CancelMarketData
            | OutgoingMessages::CancelOrder
            | OutgoingMessages::CancelMarketDepth
            | OutgoingMessages::CancelNewsBulletin
            | OutgoingMessages::CancelScannerSubscription
            | OutgoingMessages::CancelHistoricalData
            | OutgoingMessages::CancelRealTimeBars
            | OutgoingMessages::CancelFundamentalData
            | OutgoingMessages::CancelImpliedVolatility
            | OutgoingMessages::CancelOptionPrice
            | OutgoingMessages::CancelAccountSummary
            | OutgoingMessages::CancelPositions
            | OutgoingMessages::CancelPositionsMulti
            | OutgoingMessages::CancelAccountUpdatesMulti
            | OutgoingMessages::CancelHistogramData
            | OutgoingMessages::CancelHeadTimestamp
            | OutgoingMessages::CancelPnL
            | OutgoingMessages::CancelPnLSingle
            | OutgoingMessages::CancelTickByTickData
            | OutgoingMessages::CancelWshMetaData
            | OutgoingMessages::CancelWshEventData
            | OutgoingMessages::CancelContractData
            | OutgoingMessages::CancelHistoricalTicks
            | OutgoingMessages::RequestGlobalCancel
            | OutgoingMessages::UnsubscribeFromGroupEvents => true,
            OutgoingMessages::RequestMarketData
            | OutgoingMessages::PlaceOrder
            | OutgoingMessages::RequestOpenOrders
            | OutgoingMessages::RequestAccountData
            | OutgoingMessages::RequestExecutions
            | OutgoingMessages::RequestIds
            | OutgoingMessages::RequestContractData
            | OutgoingMessages::RequestMarketDepth
            | OutgoingMessages::RequestNewsBulletins
            | OutgoingMessages::ChangeServerLog
            | OutgoingMessages::RequestAutoOpenOrders
            | OutgoingMessages::RequestAllOpenOrders
            | OutgoingMessages::RequestManagedAccounts
            | OutgoingMessages::RequestFA
            | OutgoingMessages::ReplaceFA
            | OutgoingMessages::RequestHistoricalData
            | OutgoingMessages::ExerciseOptions
            | OutgoingMessages::RequestScannerSubscription
            | OutgoingMessages::RequestScannerParameters
            | OutgoingMessages::RequestCurrentTime
            | OutgoingMessages::RequestRealTimeBars
            | OutgoingMessages::RequestFundamentalData
            | OutgoingMessages::ReqCalcImpliedVolat
            | OutgoingMessages::ReqCalcOptionPrice
            | OutgoingMessages::RequestMarketDataType
            | OutgoingMessages::RequestPositions
            | OutgoingMessages::RequestAccountSummary
            | OutgoingMessages::VerifyRequest
            | OutgoingMessages::VerifyMessage
            | OutgoingMessages::QueryDisplayGroups
            | OutgoingMessages::SubscribeToGroupEvents
            | OutgoingMessages::UpdateDisplayGroup
            | OutgoingMessages::StartApi
            | OutgoingMessages::VerifyAndAuthRequest
            | OutgoingMessages::VerifyAndAuthMessage
            | OutgoingMessages::RequestPositionsMulti
            | OutgoingMessages::RequestAccountUpdatesMulti
            | OutgoingMessages::RequestSecurityDefinitionOptionalParameters
            | OutgoingMessages::RequestSoftDollarTiers
            | OutgoingMessages::RequestFamilyCodes
            | OutgoingMessages::RequestMatchingSymbols
            | OutgoingMessages::RequestMktDepthExchanges
            | OutgoingMessages::RequestSmartComponents
            | OutgoingMessages::RequestNewsArticle
            | OutgoingMessages::RequestNewsProviders
            | OutgoingMessages::RequestHistoricalNews
            | OutgoingMessages::RequestHeadTimestamp
            | OutgoingMessages::RequestHistogramData
            | OutgoingMessages::RequestMarketRule
            | OutgoingMessages::RequestPnL
            | OutgoingMessages::RequestPnLSingle
            | OutgoingMessages::RequestHistoricalTicks
            | OutgoingMessages::RequestTickByTickData
            | OutgoingMessages::RequestCompletedOrders
            | OutgoingMessages::RequestWshMetaData
            | OutgoingMessages::RequestWshEventData
            | OutgoingMessages::RequestUserInfo
            | OutgoingMessages::RequestCurrentTimeInMillis
            | OutgoingMessages::ReqConfig
            | OutgoingMessages::UpdateConfig => false,
        }
    }
}

/// Encode the outbound message length prefix using the IB wire format.
pub(crate) fn encode_length(message: &str) -> Vec<u8> {
    encode_raw_length(message.as_bytes())
}

/// Encode a protobuf outbound message: 4-byte BE (msg_id + 200) + proto bytes.
pub(crate) fn encode_protobuf_message(msg_id: i32, proto_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(MESSAGE_ID_LEN + proto_bytes.len());
    buf.write_i32::<BigEndian>(msg_id + PROTOBUF_MSG_ID).unwrap();
    buf.extend_from_slice(proto_bytes);
    buf
}

/// Split a frame body into its 4-byte BE message id, as sent (protobuf ids
/// still carry the [`PROTOBUF_MSG_ID`] offset), and the payload. `None` if the
/// body is too short to hold an id.
pub(crate) fn split_message_id(frame: &[u8]) -> Option<(i32, &[u8])> {
    let (header, payload) = frame.split_first_chunk::<MESSAGE_ID_LEN>()?;
    Some((i32::from_be_bytes(*header), payload))
}

/// The request kind of an outbound frame body, or `None` if the id is missing
/// or unknown.
pub(crate) fn outgoing_message_type(frame: &[u8]) -> Option<OutgoingMessages> {
    let (id, _) = split_message_id(frame)?;
    let id = if id > PROTOBUF_MSG_ID { id - PROTOBUF_MSG_ID } else { id };
    OutgoingMessages::try_from(id).ok()
}

/// Whether an outbound frame body is a cancel (see [`OutgoingMessages::is_cancel`]).
pub(crate) fn is_cancel_frame(frame: &[u8]) -> bool {
    outgoing_message_type(frame).is_some_and(OutgoingMessages::is_cancel)
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
/// constructors, and methods are crate-visible only. Framing is discriminated
/// by `raw_bytes`: `Some(_)` = protobuf payload, `None` = NUL-delimited text
/// payload pre-parsed into `fields` (carries WSH metadata/event-data, plus
/// unhandled text-framed message types that fall through to the dispatcher
/// catch-all).
#[derive(Clone, Default, Debug)]
pub(crate) struct ResponseMessage {
    /// Cursor index for incremental decoding.
    pub i: usize,
    /// Raw field buffer backing this message. Empty for protobuf frames, whose
    /// payload lives in `raw_bytes` and whose discriminant lives in `kind`.
    pub fields: Vec<String>,
    /// Raw protobuf payload bytes (everything after the 4-byte binary message ID).
    pub raw_bytes: Option<Vec<u8>>,
    /// The message discriminant, resolved once at construction.
    ///
    /// Read 4–6 times per inbound message — routing, request-id extraction, the
    /// subscription's `RESPONSE_MESSAGE_IDS` filter, the decoder's own match —
    /// each of which used to re-run `i32::from_str` over `fields[0]`.
    kind: IncomingMessages,
    /// The numeric id `kind` was resolved from, retained because that mapping
    /// is lossy: every unrecognized id collapses to
    /// [`IncomingMessages::NotValid`]. See [`UNKNOWN_MESSAGE_TYPE_CODE`] for
    /// what the id buys a reader that the kind alone cannot. `None` when the
    /// message carried no id at all (a text payload with a missing or
    /// unparsable `fields[0]`, or `Default`); no `i32` can stand in for that,
    /// since 0 and negative ids both reach the text path off the wire.
    message_id: Option<i32>,
}

impl ResponseMessage {
    /// The one construction path, so `kind` and `message_id` cannot disagree:
    /// `kind` is `IncomingMessages::from(id)`, or `NotValid` with no id. The
    /// derived `Default` upholds it too: `None` and `NotValid`, the enum's
    /// `#[default]`.
    fn new(message_id: Option<i32>, fields: Vec<String>, raw_bytes: Option<Vec<u8>>) -> Self {
        Self {
            i: 0,
            fields,
            raw_bytes,
            kind: message_id.map_or(IncomingMessages::NotValid, IncomingMessages::from),
            message_id,
        }
    }

    /// Build a protobuf response message from a binary message type and raw payload bytes.
    pub fn from_protobuf(message_type: i32, raw_bytes: Vec<u8>) -> Self {
        Self::new(Some(message_type), Vec::new(), Some(raw_bytes))
    }

    /// The numeric message id this frame arrived with, before it was resolved
    /// to an [`IncomingMessages`] kind. For protobuf frames this is the id with
    /// the [`PROTOBUF_MSG_ID`] offset already removed — i.e. the value that was
    /// looked up, not the raw 4 bytes off the wire.
    ///
    /// Always `Some` for protobuf frames; `None` only for a text payload
    /// whose `fields[0]` is missing or unparsable.
    ///
    /// Guaranteed for every message, not just unroutable ones:
    /// `m.message_id().map_or(NotValid, IncomingMessages::from) == m.message_type()`.
    /// Callers that need the id back out of a message — the wire recorder,
    /// diagnostics for an unrecognized frame — depend on that holding for known
    /// kinds too.
    pub(crate) fn message_id(&self) -> Option<i32> {
        self.message_id
    }

    /// Raw protobuf payload bytes, if this is a protobuf message.
    pub fn raw_bytes(&self) -> Option<&[u8]> {
        self.raw_bytes.as_deref()
    }

    /// Raw protobuf payload bytes behind [`Self::proto`]. Text-framed
    /// arrival returns `Error::UnexpectedWireFormat`, which fails the
    /// subscription — the message was addressed to this decoder, so dropping it
    /// would lose data. Nothing here is skipped: whether a message belongs to a
    /// subscription is decided from `RESPONSE_MESSAGE_IDS` before decoding. See
    /// docs/rules/wire/proto-only-decoding.md.
    fn require_proto(&self) -> Result<&[u8], crate::Error> {
        self.raw_bytes().ok_or_else(|| crate::Error::unexpected_wire_format(self))
    }

    /// The protobuf payload decoded as `M`, inferred from the decoder it is
    /// handed to. Text-framed arrival fails as in [`Self::require_proto`].
    pub(crate) fn proto<M: crate::proto::decoders::DecodeProto>(&self) -> Result<M, crate::Error> {
        M::decode_proto(self.require_proto()?)
    }

    /// Narrow a frame to one expected message type, or fail with
    /// `Error::UnexpectedResponse`.
    ///
    /// [`Self::require_proto`]'s sibling on the other axis: that one narrows the
    /// framing, this one narrows the type. Both return a borrow so they chain in
    /// the position the decoder wants.
    ///
    /// One-shot request paths should not call this directly — they compose both
    /// narrows through `request_helpers::expect_proto`, which takes the expected
    /// type alongside the payload decoder so a call site cannot supply one
    /// without the other.
    ///
    /// There is deliberately no `IncomingMessages::Error` case. The dispatcher
    /// classifies error frames before any decoder sees them, so one cannot
    /// arrive here — see `docs/rules/wire/proto-only-decoding.md`.
    pub(crate) fn expect_type(&self, expected: IncomingMessages) -> Result<&Self, Error> {
        if self.message_type() == expected {
            Ok(self)
        } else {
            Err(Error::UnexpectedResponse(format!("expected {expected:?}, got {self:?}")))
        }
    }

    /// Returns `true` if the message informs about API shutdown.
    #[cfg_attr(not(feature = "sync"), allow(dead_code))] // sync-transport-only caller
    pub fn is_shutdown(&self) -> bool {
        self.message_type() == IncomingMessages::Shutdown
    }

    /// Return the discriminator identifying the message payload.
    pub fn message_type(&self) -> IncomingMessages {
        self.kind
    }

    /// Try to extract the request id from the message.
    ///
    /// Proto-framed messages carry it at proto tag 1 in `raw_bytes`. The
    /// text-framed branch reads the per-message-type field index — at floor
    /// 213 no production message arrives text-framed, so this path is
    /// unreachable through production decoders but exercised by tests.
    /// Returns `None` for any message type not in the [`routes_by_request_id`]
    /// allow-list, even if its proto envelope happens to carry an `int32 @
    /// tag 1` for some other purpose.
    pub fn request_id(&self) -> Option<i32> {
        let kind = self.message_type();
        if !routes_by_request_id(kind) {
            return None;
        }
        if let Some(raw) = self.raw_bytes() {
            let env: ProtoIdEnvelope = prost::Message::decode(raw).ok()?;
            env.id
        } else {
            self.peek_int(text_request_id_field(kind)?).ok()
        }
    }

    /// Try to extract the order id from the message.
    ///
    /// Every `order_id`-bearing message type (`OpenOrder`, `OrderStatus`,
    /// `ExecutionData`, `ExecutionDataEnd`) is proto-framed. Three carry
    /// `order_id` at proto tag 1 (decoded via the minimal [`ProtoIdEnvelope`]);
    /// `ExecutionData` nests it under `execution.order_id` and uses
    /// [`ExecutionDetailsMinimal`] to skip the `contract` sub-message.
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
    /// `ExecutionData` nests `exec_id` under `execution.exec_id`;
    /// `CommissionsReport` carries it at the top level. Both arrive
    /// proto-framed.
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

    /// Peek an integer field without advancing the cursor. Called from
    /// [`Self::request_id`]'s text-fallback path (tests only at floor 213)
    /// and the handshake parser (pre-proto framing).
    ///
    /// Refuses proto-framed messages defensively (per docs/rules/wire/proto-aware-accessors.md):
    /// if `raw_bytes()` is `Some`, the caller picked the wrong accessor —
    /// they should be decoding the proto envelope, not reading a text-field
    /// index. Production callers (`request_id`, handshake) already gate on
    /// `raw_bytes().is_none()` so this guard is unreachable from them.
    ///
    /// This is [`Self::require_proto`]'s mirror image, and returns the same
    /// `Error::UnexpectedWireFormat` — a framing mismatch is never the
    /// skippable `UnexpectedResponse`, in either direction.
    pub fn peek_int(&self, i: usize) -> Result<i32, Error> {
        if self.raw_bytes().is_some() {
            return Err(Error::unexpected_wire_format(self));
        }
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

    /// Consume and parse the next floating-point field. Test-only symmetry
    /// with [`Self::next_int`] / [`Self::next_string`] — production code no
    /// longer reads doubles from text-framed messages at floor 213.
    #[cfg(test)]
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
        ResponseMessage::from_text_fields(fields.split_terminator('\x00').map(|x| x.to_string()).collect())
    }

    /// Text frames keep the discriminant as `fields[0]` — the handshake reader
    /// walks the field cursor from index 0 — so `kind` is derived from it here
    /// rather than replacing it.
    pub(crate) fn from_text_fields(fields: Vec<String>) -> ResponseMessage {
        let message_id = fields.first().and_then(|id| i32::from_str(id).ok());
        Self::new(message_id, fields, None)
    }

    #[cfg(test)]
    /// Build a response message from a pipe-delimited payload (test helper).
    pub fn from_simple(fields: &str) -> ResponseMessage {
        ResponseMessage::from_text_fields(fields.split_terminator('|').map(|x| x.to_string()).collect())
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
