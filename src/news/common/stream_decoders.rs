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
            IncomingMessages::NewsBulletins => Ok(decoders::decode_news_bulletin(message.clone())?),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
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
            IncomingMessages::HistoricalNews => Ok(decoders::decode_historical_news(None, message.clone())?),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            IncomingMessages::TickNews => Ok(decoders::decode_tick_news(message.clone())?),
            IncomingMessages::Error => Err(Error::from(message.clone())),
            _ => Err(Error::UnexpectedResponse(message.clone())),
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
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_tws_error_message;

    fn test_context() -> DecoderContext {
        DecoderContext::new(176, None)
    }

    fn error_message() -> ResponseMessage {
        ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|")
    }

    #[test]
    fn test_news_bulletin_decode_error_message() {
        // Error on the request_id channel surfaces as Error::Message, not silently
        // skipped via UnexpectedResponse (#434).
        let mut message = error_message();
        let err = NewsBulletin::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }

    #[test]
    fn test_news_article_decode_error_message() {
        let mut message = error_message();
        let err = NewsArticle::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}
