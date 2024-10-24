use super::{IncomingMessages, OutgoingMessages};

pub struct ChannelMapping<'a> {
    pub request: OutgoingMessages,
    pub responses: &'a [IncomingMessages],
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
        responses: &[IncomingMessages::OpenOrder, IncomingMessages::OpenOrderEnd],
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
];
