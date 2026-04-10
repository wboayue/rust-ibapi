use crate::market_data::realtime;
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::news::common::decoders;
use crate::news::common::encoders;
use crate::news::{NewsArticle, NewsBulletin};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<NewsBulletin> for NewsBulletin {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::NewsBulletins];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => Ok(decoders::decode_news_bulletin(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_news_bulletin()
    }
}

impl StreamDecoder<NewsArticle> for NewsArticle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::HistoricalNews,
        IncomingMessages::HistoricalNewsEnd,
        IncomingMessages::TickNews,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<NewsArticle, Error> {
        match message.message_type() {
            IncomingMessages::HistoricalNews => Ok(decoders::decode_historical_news(None, message.clone())?),
            IncomingMessages::HistoricalNewsEnd => Err(Error::EndOfStream),
            IncomingMessages::TickNews => Ok(decoders::decode_tick_news(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        if context.and_then(|ctx| ctx.request_type) == Some(OutgoingMessages::RequestMarketData) {
            let request_id =
                request_id.ok_or_else(|| Error::InvalidArgument("request id required to cancel market data subscription".to_string()))?;
            realtime::common::encoders::encode_cancel_market_data(request_id)
        } else {
            Err(Error::NotImplemented)
        }
    }
}
