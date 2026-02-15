use super::{IncomingMessages, OutgoingMessages};

#[allow(dead_code)]
pub struct ChannelMapping {
    pub request: OutgoingMessages,
    pub responses: &'static [IncomingMessages],
}

// For shared channels configures mapping of request message id to response message ids.
pub(crate) const CHANNEL_MAPPINGS: &[ChannelMapping] = &[
    ChannelMapping {
        request: OutgoingMessages::RequestIds,
        responses: &[IncomingMessages::NextValidId],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestFamilyCodes,
        responses: &[IncomingMessages::FamilyCodes],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMarketRule,
        responses: &[IncomingMessages::MarketRule],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestPositions,
        responses: &[IncomingMessages::Position, IncomingMessages::PositionEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestPositionsMulti,
        responses: &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAllOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAutoOpenOrders,
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::OpenOrderEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCompletedOrders,
        responses: &[IncomingMessages::CompletedOrder, IncomingMessages::CompletedOrdersEnd],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestManagedAccounts,
        responses: &[IncomingMessages::ManagedAccounts],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestAccountData,
        responses: &[
            IncomingMessages::AccountValue,
            IncomingMessages::PortfolioValue,
            IncomingMessages::AccountDownloadEnd,
            IncomingMessages::AccountUpdateTime,
        ],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMarketDataType,
        responses: &[IncomingMessages::MarketDataType],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestMktDepthExchanges,
        responses: &[IncomingMessages::MktDepthExchanges],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCurrentTime,
        responses: &[IncomingMessages::CurrentTime],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestCurrentTimeInMillis,
        responses: &[IncomingMessages::CurrentTimeInMillis],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestNewsProviders,
        responses: &[IncomingMessages::NewsProviders],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestNewsBulletins,
        responses: &[IncomingMessages::NewsBulletins],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestScannerParameters,
        responses: &[IncomingMessages::ScannerParameters],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestWshMetaData,
        responses: &[IncomingMessages::WshMetaData],
    },
    ChannelMapping {
        request: OutgoingMessages::RequestWshEventData,
        responses: &[IncomingMessages::WshEventData],
    },
];
