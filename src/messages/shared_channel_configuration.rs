use std::collections::HashSet;
use std::sync::LazyLock;

use super::{IncomingMessages, OutgoingMessages};

#[allow(dead_code)]
pub struct ChannelMapping {
    pub request: OutgoingMessages,
    pub responses: &'static [IncomingMessages],
    /// `true` when the request expects a single terminating response (a one-shot
    /// like `NextValidId` / `MarketRule`), `false` for ongoing streams (positions,
    /// open orders, account updates, news bulletins, WSH events).
    ///
    /// Drives fail-fast delivery of request-less errors: a TWS error with no
    /// request id (`id == -1`) cannot be correlated to a specific shared request,
    /// so it is delivered to in-flight *one-shot* channels only. Streaming
    /// channels are excluded because an unrelated error would otherwise terminate
    /// a live subscription. See [`one_shot_error_response_types`].
    ///
    /// Note this is explicit data, not derived from the presence of an `*End`
    /// response: `NewsBulletins` and `WshEventData` stream without an End marker.
    pub one_shot: bool,
}

// For shared channels configures mapping of request message id to response message ids.
pub(crate) const CHANNEL_MAPPINGS: &[ChannelMapping] = &[
    ChannelMapping {
        request: OutgoingMessages::RequestIds,
        responses: &[IncomingMessages::NextValidId],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestFamilyCodes,
        responses: &[IncomingMessages::FamilyCodes],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMarketRule,
        responses: &[IncomingMessages::MarketRule],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestPositions,
        responses: &[IncomingMessages::Position, IncomingMessages::PositionEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestPositionsMulti,
        responses: &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAllOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAutoOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCompletedOrders,
        responses: &[IncomingMessages::CompletedOrder, IncomingMessages::CompletedOrdersEnd],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestManagedAccounts,
        responses: &[IncomingMessages::ManagedAccounts],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAccountData,
        responses: &[
            IncomingMessages::AccountValue,
            IncomingMessages::PortfolioValue,
            IncomingMessages::AccountDownloadEnd,
            IncomingMessages::AccountUpdateTime,
        ],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMarketDataType,
        responses: &[IncomingMessages::MarketDataType],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMktDepthExchanges,
        responses: &[IncomingMessages::MktDepthExchanges],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCurrentTime,
        responses: &[IncomingMessages::CurrentTime],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCurrentTimeInMillis,
        responses: &[IncomingMessages::CurrentTimeInMillis],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestNewsProviders,
        responses: &[IncomingMessages::NewsProviders],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestNewsBulletins,
        responses: &[IncomingMessages::NewsBulletins],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestScannerParameters,
        responses: &[IncomingMessages::ScannerParameters],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestWshMetaData,
        responses: &[IncomingMessages::WshMetaData],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestWshEventData,
        responses: &[IncomingMessages::WshEventData],
        one_shot: false,
    },
    ChannelMapping {
        request: OutgoingMessages::RequestFA,
        responses: &[IncomingMessages::ReceiveFA],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::VerifyRequest,
        responses: &[IncomingMessages::VerifyMessageApi],
        one_shot: true,
    },
    ChannelMapping {
        request: OutgoingMessages::VerifyMessage,
        responses: &[IncomingMessages::VerifyCompleted],
        one_shot: true,
    },
];

/// Response message types that belong **exclusively** to one-shot shared-channel
/// mappings. A request-less (`id == -1`) hard error is delivered to the senders
/// registered for these types so an awaiting one-shot call fails fast instead of
/// hanging. The set-difference excludes any type also used by a streaming mapping,
/// so a live stream is never terminated by an unrelated error.
static ONE_SHOT_ERROR_RESPONSE_TYPES: LazyLock<HashSet<IncomingMessages>> = LazyLock::new(|| {
    let streaming: HashSet<IncomingMessages> = CHANNEL_MAPPINGS
        .iter()
        .filter(|m| !m.one_shot)
        .flat_map(|m| m.responses.iter().copied())
        .collect();
    CHANNEL_MAPPINGS
        .iter()
        .filter(|m| m.one_shot)
        .flat_map(|m| m.responses.iter().copied())
        .filter(|r| !streaming.contains(r))
        .collect()
});

/// The one-shot response types eligible for fail-fast delivery of request-less
/// errors. See [`ONE_SHOT_ERROR_RESPONSE_TYPES`].
pub(crate) fn one_shot_error_response_types() -> &'static HashSet<IncomingMessages> {
    &ONE_SHOT_ERROR_RESPONSE_TYPES
}

/// `true` when `request` maps to a one-shot shared channel (single terminating
/// response). Requests without a shared-channel mapping return `false`.
///
/// Used by the sync transport to decide whether `send_shared_request` should
/// drain the shared queue before writing: only one-shot channels receive
/// fanned request-less errors, and draining a streaming channel could discard
/// messages buffered for a concurrent live subscription of the same type.
#[cfg(any(feature = "sync", test))]
pub(crate) fn is_one_shot_request(request: OutgoingMessages) -> bool {
    CHANNEL_MAPPINGS.iter().any(|m| m.request == request && m.one_shot)
}
