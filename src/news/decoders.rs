use std::str;

use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, OffsetDateTimeExt, OffsetResult, PrimitiveDateTimeExt, Tz};

use super::{ArticleType, Error, HistoricalNews, NewsArticle, NewsBulletin, NewsProvider};
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

pub(super) fn decode_historical_news(time_zone: Option<&'static Tz>, mut message: ResponseMessage) -> Result<HistoricalNews, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let time = message.next_string()?;
    let time = parse_time(time_zone, &time);

    Ok(HistoricalNews {
        time,
        provider_code: message.next_string()?,
        article_id: message.next_string()?,
        headline: message.next_string()?,
    })
}

fn parse_time(time_zone: Option<&'static Tz>, time: &str) -> OffsetDateTime {
    let timezone = time_zone.unwrap_or(timezones::db::UTC);

    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let time = PrimitiveDateTime::parse(time, format).unwrap();

    time.assume_timezone(timezone).unwrap()
}

pub(super) fn decode_news_article(mut message: ResponseMessage) -> Result<NewsArticle, Error> {
    message.skip(); // message type
    message.skip(); // request id

    Ok(NewsArticle {
        article_type: ArticleType::from(message.next_int()?),
        article_text: message.next_string()?,
    })
}
