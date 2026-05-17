use crate::market_data::realtime;
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::news::common::decoders;
use crate::news::common::encoders;
use crate::news::{NewsArticle, NewsBulletin};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<NewsBulletin> for NewsBulletin {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::NewsBulletins, IncomingMessages::Error];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => decoders::decode_news_bulletin(message),
            IncomingMessages::Error => Err(Error::from(message.clone())),
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
        IncomingMessages::Error,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsArticle, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => decoders::decode_historical_news(message),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            IncomingMessages::TickNews => decoders::decode_tick_news(message.clone()),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::unexpected_response(message)),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: Option<&DecoderContext>) -> Result<Vec<u8>, Error> {
        if context.and_then(|ctx| ctx.request_type) == Some(OutgoingMessages::RequestMarketData) {
            let request_id =
                request_id.ok_or_else(|| Error::InvalidArgument("request id required to cancel market data subscription".to_string()))?;
            realtime::common::encoders::encode_cancel_market_data(request_id)
        } else {
            Err(Error::NotImplemented)
        }
    }
}

#[cfg(test)]
#[path = "stream_decoders_tests.rs"]
mod tests;
