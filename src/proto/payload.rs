//! The message type a protobuf payload belongs to.
//!
//! A one-shot request reads one frame off a shared channel, so it must check
//! that the frame is the one it asked for before handing the bytes to a `prost`
//! type. Which `IncomingMessages` variant a payload belongs to is a property of
//! the payload, not of the caller — this is where it is declared.
//!
//! It used to be declared at the call site instead — the expected variant and
//! the decoder passed side by side as two arguments, unrelated to the compiler:
//! `R` was inferred from the decoder and the variant was a free runtime value,
//! so a mispairing built and ran and fed one message's bytes to another's
//! `prost` type. A hand-listed roster and a source-scraping test
//! (`one_shot_pairing_tests.rs`, deleted with this) stood in for the check.
//! Now `expect_proto(decode_user_info_proto)` names no message type at all: it
//! comes from the payload the decoder accepts.
//!
//! Adding a one-shot API means implementing this trait for its payload — which
//! `expect_proto` will demand, since it cannot infer `MESSAGE_ID` otherwise.

use crate::messages::IncomingMessages;

/// A protobuf message that arrives under exactly one [`IncomingMessages`] type.
pub(crate) trait ProtoPayload: prost::Message + Default {
    /// The frame type carrying this payload.
    const MESSAGE_ID: IncomingMessages;
}

/// One impl per payload. Twenty-five identical shapes with nothing but the two
/// names varying is the case [macros last resort](../../docs/rules/style/macros-last-resort.md)
/// allows — and unlike the roster it replaces, a line here is *load-bearing*:
/// `expect_proto` will not compile for a payload that lacks one.
macro_rules! impl_proto_payload {
    ($($payload:ident => $message_id:ident),+ $(,)?) => {
        $(
            impl ProtoPayload for crate::proto::$payload {
                const MESSAGE_ID: IncomingMessages = IncomingMessages::$message_id;
            }
        )+
    };
}

// Left: the generated `prost` type. Right: the wire message it arrives under.
// Most read as the same name twice; the three that do not are real — TWS
// abbreviates the message where the `.proto` file spells it out.
impl_proto_payload! {
    ConfigResponse => ConfigResponse,
    CurrentTime => CurrentTime,
    CurrentTimeInMillis => CurrentTimeInMillis,
    FamilyCodes => FamilyCodes,
    HeadTimestamp => HeadTimestamp,
    HistogramData => HistogramData,
    HistoricalSchedule => HistoricalSchedule,
    ManagedAccounts => ManagedAccounts,
    MarketDepthExchanges => MktDepthExchanges,
    MarketRule => MarketRule,
    NewsArticle => NewsArticle,
    NewsProviders => NewsProviders,
    NextValidId => NextValidId,
    ReceiveFa => ReceiveFA,
    ReplaceFaEnd => ReplaceFAEnd,
    ScannerParameters => ScannerParameters,
    SmartComponents => SmartComponents,
    SoftDollarTiers => SoftDollarTier,
    SymbolSamples => SymbolSamples,
    UpdateConfigResponse => UpdateConfigResponse,
    UserInfo => UserInfo,
    VerifyCompleted => VerifyCompleted,
    VerifyMessageApi => VerifyMessageApi,
    WshEventData => WshEventData,
    WshMetaData => WshMetaData,
}
