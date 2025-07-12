use std::str;

use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, PrimitiveDateTimeExt, Tz};

use super::super::{ArticleType, NewsArticle, NewsArticleBody, NewsBulletin, NewsProvider};
use crate::messages::ResponseMessage;
use crate::Error;

pub(in crate::news) fn decode_news_providers(mut message: ResponseMessage) -> Result<Vec<NewsProvider>, Error> {
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

pub(in crate::news) fn decode_news_bulletin(mut message: ResponseMessage) -> Result<NewsBulletin, Error> {
    message.skip(); // message type
    message.skip(); // message version

    Ok(NewsBulletin {
        message_id: message.next_int()?,
        message_type: message.next_int()?,
        message: message.next_string()?,
        exchange: message.next_string()?,
    })
}

pub(in crate::news) fn decode_historical_news(_time_zone: Option<&'static Tz>, mut message: ResponseMessage) -> Result<NewsArticle, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let time = message.next_string()?;
    let time = parse_time_as_utc(&time);

    Ok(NewsArticle {
        time,
        provider_code: message.next_string()?,
        article_id: message.next_string()?,
        headline: message.next_string()?,
        extra_data: "".to_string(),
    })
}

fn parse_time_as_utc(time: &str) -> OffsetDateTime {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let time = PrimitiveDateTime::parse(time, format).unwrap();

    time.assume_timezone(timezones::db::UTC).unwrap()
}

pub(in crate::news) fn decode_news_article(mut message: ResponseMessage) -> Result<NewsArticleBody, Error> {
    message.skip(); // message type
    message.skip(); // request id

    Ok(NewsArticleBody {
        article_type: ArticleType::from(message.next_int()?),
        article_text: message.next_string()?,
    })
}

pub(in crate::news) fn decode_tick_news(mut message: ResponseMessage) -> Result<NewsArticle, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let time = message.next_string()?;
    let time = parse_unix_timestamp(&time)?;

    Ok(NewsArticle {
        time,
        provider_code: message.next_string()?,
        article_id: message.next_string()?,
        headline: message.next_string()?,
        extra_data: message.next_string()?,
    })
}

fn parse_unix_timestamp(time: &str) -> Result<OffsetDateTime, Error> {
    let time: i64 = time.parse()?;
    let time = time / 1000;

    match OffsetDateTime::from_unix_timestamp(time) {
        Ok(val) => Ok(val),
        Err(err) => Err(Error::Simple(err.to_string())),
    }
}
