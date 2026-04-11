use std::str;

use prost::Message;
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
    let time: i64 = time
        .parse()
        .map_err(|e: std::num::ParseIntError| Error::Simple(format!("parse error: \"{time}\" - {e}")))?;
    let time = time / 1000;

    match OffsetDateTime::from_unix_timestamp(time) {
        Ok(val) => Ok(val),
        Err(err) => Err(Error::Simple(err.to_string())),
    }
}

#[allow(dead_code)]
pub(crate) fn decode_news_bulletin_proto(bytes: &[u8]) -> Result<NewsBulletin, Error> {
    let p = crate::proto::NewsBulletin::decode(bytes)?;
    Ok(NewsBulletin {
        message_id: p.news_msg_id.unwrap_or_default(),
        message_type: p.news_msg_type.unwrap_or_default(),
        message: p.news_message.unwrap_or_default(),
        exchange: p.originating_exch.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_news_article_proto(bytes: &[u8]) -> Result<NewsArticleBody, Error> {
    let p = crate::proto::NewsArticle::decode(bytes)?;
    Ok(NewsArticleBody {
        article_type: ArticleType::from(p.article_type.unwrap_or_default()),
        article_text: p.article_text.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_historical_news_proto(bytes: &[u8]) -> Result<NewsArticle, Error> {
    let p = crate::proto::HistoricalNews::decode(bytes)?;

    let time = p
        .time
        .as_deref()
        .and_then(|t| {
            let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
            PrimitiveDateTime::parse(t, format).ok()
        })
        .and_then(|dt| match dt.assume_timezone(timezones::db::UTC) {
            time_tz::OffsetResult::Some(v) => Some(v),
            _ => None,
        })
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);

    Ok(NewsArticle {
        time,
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn test_parse_unix_timestamp() {
        let result = parse_unix_timestamp("1681133400000").unwrap();
        assert_eq!(result, datetime!(2023-04-10 13:30:00 UTC));
    }

    #[test]
    fn test_parse_unix_timestamp_invalid() {
        let err = parse_unix_timestamp("not_a_number").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not_a_number"), "error should include the bad value: {msg}");
        assert!(msg.contains("invalid digit"), "error should include parse reason: {msg}");
    }

    #[test]
    fn test_decode_news_bulletin_proto() {
        use prost::Message;

        let proto_msg = crate::proto::NewsBulletin {
            news_msg_id: Some(42),
            news_msg_type: Some(1),
            news_message: Some("Market closed early".into()),
            originating_exch: Some("NYSE".into()),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_news_bulletin_proto(&bytes).unwrap();
        assert_eq!(result.message_id, 42);
        assert_eq!(result.message_type, 1);
        assert_eq!(result.message, "Market closed early");
        assert_eq!(result.exchange, "NYSE");
    }

    #[test]
    fn test_decode_news_article_proto() {
        use prost::Message;

        let proto_msg = crate::proto::NewsArticle {
            req_id: Some(1),
            article_type: Some(0),
            article_text: Some("Full article text here".into()),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_news_article_proto(&bytes).unwrap();
        assert_eq!(result.article_type, ArticleType::Text);
        assert_eq!(result.article_text, "Full article text here");
    }

    #[test]
    fn test_decode_historical_news_proto() {
        use prost::Message;

        let proto_msg = crate::proto::HistoricalNews {
            req_id: Some(1),
            time: Some("2023-04-10 13:30:00.000".into()),
            provider_code: Some("BRFG".into()),
            article_id: Some("BRFG$12345".into()),
            headline: Some("Market Update".into()),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_historical_news_proto(&bytes).unwrap();
        assert_eq!(result.provider_code, "BRFG");
        assert_eq!(result.article_id, "BRFG$12345");
        assert_eq!(result.headline, "Market Update");
        assert_ne!(result.time, OffsetDateTime::UNIX_EPOCH);
    }
}
