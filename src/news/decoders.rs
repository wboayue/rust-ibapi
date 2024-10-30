use time::OffsetDateTime;

use super::{Error, HistoricalNews, NewsBulletin, NewsProvider};
use crate::messages::ResponseMessage;

pub(super) fn decode_news_providers(mut message: ResponseMessage) -> Result<Vec<NewsProvider>, Error> {
    message.skip(); // message type

    let num_providers = message.next_int()?;
    let mut news_providers = Vec::with_capacity(num_providers as usize);

    for _ in 0..num_providers {
        news_providers.push(NewsProvider {
            code: message.next_string()?,
            name: message.next_string()?,
        });
    }

    Ok(news_providers)
}

pub(super) fn decode_news_bulletin(mut message: ResponseMessage) -> Result<NewsBulletin, Error> {
    message.skip(); // message type
    message.skip(); // message version

    Ok(NewsBulletin {
        message_id: message.next_int()?,
        message_type: message.next_int()?,
        message: message.next_string()?,
        exchange: message.next_string()?,
    })
}

pub(super) fn decode_historical_news(mut message: ResponseMessage) -> Result<HistoricalNews, Error> {
    message.skip(); // message type
    message.skip(); // request id
    message.skip(); // time

    Ok(HistoricalNews {
        time: OffsetDateTime::now_utc(),
        provider_code: message.next_string()?,
        article_id: message.next_string()?,
        headline: message.next_string()?,
    })
}
