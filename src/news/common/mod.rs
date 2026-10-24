pub(super) mod decoders;
pub(super) mod encoders;
pub(crate) mod stream_decoders;

use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::news::HistoricalNewsPage;
use crate::subscriptions::DecoderContext;
use crate::Error;

/// Folds one frame of a `reqHistoricalNews` reply into `page`. `Ok(true)`
/// once the end frame has been read.
pub(crate) fn historical_news_page_step(page: &mut HistoricalNewsPage, message: &ResponseMessage) -> Result<bool, Error> {
    match message.message_type() {
        IncomingMessages::HistoricalNews => {
            page.articles.push(decoders::decode_historical_news(message)?);
            Ok(false)
        }
        IncomingMessages::HistoricalNewsEnd => {
            page.has_more = decoders::decode_historical_news_end(message)?;
            Ok(true)
        }
        _ => Err(Error::unexpected_response(message)),
    }
}

/// Decoder context for news subscribed through `reqMktData` (generic tick 292),
/// so `NewsArticle`'s cancel sends `cancelMktData`.
pub(crate) fn tick_news_context(context: DecoderContext) -> DecoderContext {
    context.with_request_type(OutgoingMessages::RequestMarketData)
}
