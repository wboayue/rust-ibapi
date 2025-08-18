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

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum IncomingMessages {
    Shutdown = -2,
    NotValid = -1,
    TickPrice = 1,
    TickSize = 2,
    OrderStatus = 3,
    Error = 4,
    OpenOrder = 5,
    AccountValue = 6,
    PortfolioValue = 7,
    AccountUpdateTime = 8,
    NextValidId = 9,
    ContractData = 10,
    ExecutionData = 11,
    MarketDepth = 12,
    MarketDepthL2 = 13,
    NewsBulletins = 14,
    ManagedAccounts = 15,
    ReceiveFA = 16,
    HistoricalData = 17,
    BondContractData = 18,
    ScannerParameters = 19,
    ScannerData = 20,
    TickOptionComputation = 21,
    TickGeneric = 45,
    TickString = 46,
    TickEFP = 47, //TICK EFP 47
    CurrentTime = 49,
    RealTimeBars = 50,
    FundamentalData = 51,
    ContractDataEnd = 52,
    OpenOrderEnd = 53,
    AccountDownloadEnd = 54,
    ExecutionDataEnd = 55,
    DeltaNeutralValidation = 56,
    TickSnapshotEnd = 57,
    MarketDataType = 58,
    CommissionsReport = 59,
    Position = 61,
    PositionEnd = 62,
    AccountSummary = 63,
    AccountSummaryEnd = 64,
    VerifyMessageApi = 65,
    VerifyCompleted = 66,
    DisplayGroupList = 67,
    DisplayGroupUpdated = 68,
    VerifyAndAuthMessageApi = 69,
    VerifyAndAuthCompleted = 70,
    PositionMulti = 71,
    PositionMultiEnd = 72,
    AccountUpdateMulti = 73,
    AccountUpdateMultiEnd = 74,
    SecurityDefinitionOptionParameter = 75,
    SecurityDefinitionOptionParameterEnd = 76,
    SoftDollarTier = 77,
    FamilyCodes = 78,
    SymbolSamples = 79,
    MktDepthExchanges = 80,
    TickReqParams = 81,
    SmartComponents = 82,
    NewsArticle = 83,
    TickNews = 84,
    NewsProviders = 85,
    HistoricalNews = 86,
    HistoricalNewsEnd = 87,
    HeadTimestamp = 88,
    HistogramData = 89,
    HistoricalDataUpdate = 90,
    RerouteMktDataReq = 91,
    RerouteMktDepthReq = 92,
    MarketRule = 93,
    PnL = 94,
    PnLSingle = 95,
    HistoricalTick = 96,
    HistoricalTickBidAsk = 97,
    HistoricalTickLast = 98,
    TickByTick = 99,
    OrderBound = 100,
    CompletedOrder = 101,
    CompletedOrdersEnd = 102,
    ReplaceFAEnd = 103,
    WshMetaData = 104,
    WshEventData = 105,
    HistoricalSchedule = 106,
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

pub fn order_id_index(kind: IncomingMessages) -> Option<usize> {
    match kind {
        IncomingMessages::OpenOrder | IncomingMessages::OrderStatus => Some(1),
        IncomingMessages::ExecutionData | IncomingMessages::ExecutionDataEnd => Some(2),
        _ => None,
    }
}

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

        _ => {
            debug!("could not determine request id index for {kind:?} (this message type may not have a request id).");
            None
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum OutgoingMessages {
    RequestMarketData = 1,
    CancelMarketData = 2,
    PlaceOrder = 3,
    CancelOrder = 4,
    RequestOpenOrders = 5,
    RequestAccountData = 6,
    RequestExecutions = 7,
    RequestIds = 8,
    RequestContractData = 9,
    RequestMarketDepth = 10,
    CancelMarketDepth = 11,
    RequestNewsBulletins = 12,
    CancelNewsBulletin = 13,
    ChangeServerLog = 14,
    RequestAutoOpenOrders = 15,
    RequestAllOpenOrders = 16,
    RequestManagedAccounts = 17,
    RequestFA = 18,
    ReplaceFA = 19,
    RequestHistoricalData = 20,
    ExerciseOptions = 21,
    RequestScannerSubscription = 22,
    CancelScannerSubscription = 23,
    RequestScannerParameters = 24,
    CancelHistoricalData = 25,
    RequestCurrentTime = 49,
    RequestRealTimeBars = 50,
    CancelRealTimeBars = 51,
    RequestFundamentalData = 52,
    CancelFundamentalData = 53,
    ReqCalcImpliedVolat = 54,
    ReqCalcOptionPrice = 55,
    CancelImpliedVolatility = 56,
    CancelOptionPrice = 57,
    RequestGlobalCancel = 58,
    RequestMarketDataType = 59,
    RequestPositions = 61,
    RequestAccountSummary = 62,
    CancelAccountSummary = 63,
    CancelPositions = 64,
    VerifyRequest = 65,
    VerifyMessage = 66,
    QueryDisplayGroups = 67,
    SubscribeToGroupEvents = 68,
    UpdateDisplayGroup = 69,
    UnsubscribeFromGroupEvents = 70,
    StartApi = 71,
    VerifyAndAuthRequest = 72,
    VerifyAndAuthMessage = 73,
    RequestPositionsMulti = 74,
    CancelPositionsMulti = 75,
    RequestAccountUpdatesMulti = 76,
    CancelAccountUpdatesMulti = 77,
    RequestSecurityDefinitionOptionalParameters = 78,
    RequestSoftDollarTiers = 79,
    RequestFamilyCodes = 80,
    RequestMatchingSymbols = 81,
    RequestMktDepthExchanges = 82,
    RequestSmartComponents = 83,
    RequestNewsArticle = 84,
    RequestNewsProviders = 85,
    RequestHistoricalNews = 86,
    RequestHeadTimestamp = 87,
    RequestHistogramData = 88,
    CancelHistogramData = 89,
    CancelHeadTimestamp = 90,
    RequestMarketRule = 91,
    RequestPnL = 92,
    CancelPnL = 93,
    RequestPnLSingle = 94,
    CancelPnLSingle = 95,
    RequestHistoricalTicks = 96,
    RequestTickByTickData = 97,
    CancelTickByTickData = 98,
    RequestCompletedOrders = 99,
    RequestWshMetaData = 100,
    CancelWshMetaData = 101,
    RequestWshEventData = 102,
    CancelWshEventData = 103,
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

pub fn encode_length(message: &str) -> Vec<u8> {
    let data = message.as_bytes();

    let mut packet: Vec<u8> = Vec::with_capacity(data.len() + 4);

    packet.write_u32::<BigEndian>(data.len() as u32).unwrap();
    packet.write_all(data).unwrap();
    packet
}

#[derive(Default, Debug, Clone)]
pub struct RequestMessage {
    pub(crate) fields: Vec<String>,
}

impl RequestMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_field<T: ToField>(&mut self, val: &T) -> &RequestMessage {
        let field = val.to_field();
        self.fields.push(field);
        self
    }

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
    pub(crate) fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }
    #[cfg(test)]
    pub fn from(fields: &str) -> RequestMessage {
        RequestMessage {
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
        }
    }
    #[cfg(test)]
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

#[derive(Clone, Default, Debug)]
pub struct ResponseMessage {
    pub i: usize,
    pub fields: Vec<String>,
}

impl ResponseMessage {
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn is_shutdown(&self) -> bool {
        self.message_type() == IncomingMessages::Shutdown
    }

    pub fn message_type(&self) -> IncomingMessages {
        if self.fields.is_empty() {
            IncomingMessages::NotValid
        } else {
            let message_id = i32::from_str(&self.fields[0]).unwrap_or(-1);
            IncomingMessages::from(message_id)
        }
    }

    pub fn request_id(&self) -> Option<i32> {
        if let Some(i) = request_id_index(self.message_type()) {
            if let Ok(request_id) = self.peek_int(i) {
                return Some(request_id);
            }
        }
        None
    }

    pub fn order_id(&self) -> Option<i32> {
        if let Some(i) = order_id_index(self.message_type()) {
            if let Ok(order_id) = self.peek_int(i) {
                return Some(order_id);
            }
        }
        None
    }

    pub fn execution_id(&self) -> Option<String> {
        match self.message_type() {
            IncomingMessages::ExecutionData => Some(self.peek_string(14)),
            IncomingMessages::CommissionsReport => Some(self.peek_string(2)),
            _ => None,
        }
    }

    pub fn peek_int(&self, i: usize) -> Result<i32, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected int and found end of message".into()));
        }

        let field = &self.fields[i];
        match field.parse() {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Parse(i, field.into(), err.to_string())),
        }
    }

    pub fn peek_string(&self, i: usize) -> String {
        self.fields[i].to_owned()
    }

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

    pub fn next_bool(&mut self) -> Result<bool, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected bool and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;

        Ok(field == "1")
    }

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

    pub fn next_string(&mut self) -> Result<String, Error> {
        if self.i >= self.fields.len() {
            return Err(Error::Simple("expected string and found end of message".into()));
        }

        let field = &self.fields[self.i];
        self.i += 1;
        Ok(String::from(field))
    }

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

    pub fn from(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('\x00').map(|x| x.to_string()).collect(),
        }
    }
    #[cfg(test)]
    pub fn from_simple(fields: &str) -> ResponseMessage {
        ResponseMessage {
            i: 0,
            fields: fields.split_terminator('|').map(|x| x.to_string()).collect(),
        }
    }

    pub fn skip(&mut self) {
        self.i += 1;
    }

    pub fn encode(&self) -> String {
        let mut data = self.fields.join("\0");
        data.push('\0');
        data
    }

    #[cfg(test)]
    pub fn encode_simple(&self) -> String {
        let mut data = self.fields.join("|");
        data.push('|');
        data
    }
}

/// An error message from the TWS API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notice {
    pub code: i32,
    pub message: String,
}

impl Notice {
    #[allow(private_interfaces)]
    pub fn from(message: &ResponseMessage) -> Notice {
        let code = message.peek_int(CODE_INDEX).unwrap_or(-1);
        let message = message.peek_string(MESSAGE_INDEX);
        Notice { code, message }
    }
}

impl Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}
