use crate::common::error_helpers;
use crate::market_data::realtime;
use crate::messages::Notice;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::news::common::decoders;
use crate::news::common::encoders;
use crate::news::{HistoricalNewsFrame, NewsArticle, NewsBulletin};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<NewsBulletin> for NewsBulletin {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::NewsBulletins];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => decoders::decode_news_bulletin(message),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        encoders::encode_cancel_news_bulletin()
    }
}

impl StreamDecoder<NewsArticle> for NewsArticle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::HistoricalNews,
        IncomingMessages::HistoricalNewsEnd,
        IncomingMessages::TickNews,
    ];
    const END_MESSAGES: &'static [IncomingMessages] = &[IncomingMessages::HistoricalNewsEnd];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<NewsArticle, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => decoders::decode_historical_news(message),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            IncomingMessages::TickNews => decoders::decode_tick_news(message),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        if context.and_then(|ctx| ctx.request_type) == Some(OutgoingMessages::RequestMarketData) {
            let request_id = error_helpers::require_request_id_for(request_id, "cancel market data subscription")?;
            realtime::common::encode_cancel_market_data(request_id)
        } else {
            Err(Error::NotImplemented)
        }
    }
}

impl StreamDecoder<HistoricalNewsFrame> for HistoricalNewsFrame {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::HistoricalNews, IncomingMessages::HistoricalNewsEnd];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => Ok(Self::Article(decoders::decode_historical_news(message)?)),
            IncomingMessages::HistoricalNewsEnd => Ok(Self::End {
                has_more: decoders::decode_historical_news_end(message)?,
            }),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    // No cancel: TWS has no cancel for `reqHistoricalNews`, so `cancel_message`
    // keeps its `NotImplemented` default.

    fn is_snapshot_end(&self) -> bool {
        matches!(self, Self::End { .. })
    }

    /// Any error: a one-shot query has nothing to continue after one.
    fn rejection_ends_request(_notice: &Notice) -> bool {
        true
    }
}
