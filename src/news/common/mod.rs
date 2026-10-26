pub(super) mod decoders;
pub(super) mod encoders;
pub(crate) mod stream_decoders;

use crate::messages::OutgoingMessages;
use crate::subscriptions::DecoderContext;

/// Decoder context for news subscribed through `reqMktData` (generic tick 292),
/// so `NewsArticle`'s cancel sends `cancelMktData`.
pub(crate) fn tick_news_context(context: DecoderContext) -> DecoderContext {
    context.with_request_type(OutgoingMessages::RequestMarketData)
}
