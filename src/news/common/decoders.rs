use prost::Message;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, PrimitiveDateTimeExt};

use super::super::{ArticleType, NewsArticle, NewsArticleBody, NewsBulletin, NewsProvider};
use crate::messages::ResponseMessage;
use crate::Error;

// All originating outgoing-request gates for the news-domain messages
// (`PROTOBUF_MARKET_DATA` = 206 for `TickNews`, `PROTOBUF_NEWS_DATA` = 209 for
// the rest) sit at or below the connection floor (`PROTOBUF_SCAN_DATA` = 210),
// so the server always emits proto framing for these messages — text-framed
// arrival is rejected via `ResponseMessage::require_proto` and skip-classifies
// (rule 20).

pub(in crate::news) fn decode_news_providers(message: &ResponseMessage) -> Result<Vec<NewsProvider>, Error> {
    decode_news_providers_proto(message.require_proto()?)
}

pub(crate) fn decode_news_providers_proto(bytes: &[u8]) -> Result<Vec<NewsProvider>, Error> {
    let p = crate::proto::NewsProviders::decode(bytes)?;
    Ok(p.news_providers
        .into_iter()
        .map(|np| NewsProvider {
            code: np.provider_code.unwrap_or_default(),
            name: np.provider_name.unwrap_or_default(),
        })
        .collect())
}

pub(in crate::news) fn decode_news_bulletin(message: &ResponseMessage) -> Result<NewsBulletin, Error> {
    decode_news_bulletin_proto(message.require_proto()?)
}

pub(in crate::news) fn decode_historical_news(message: &ResponseMessage) -> Result<NewsArticle, Error> {
    decode_historical_news_proto(message.require_proto()?)
}

fn try_parse_time_as_utc(time: &str) -> Option<OffsetDateTime> {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    let dt = PrimitiveDateTime::parse(time, format).ok()?;
    match dt.assume_timezone(timezones::db::UTC) {
        time_tz::OffsetResult::Some(v) => Some(v),
        _ => None,
    }
}

pub(in crate::news) fn decode_news_article(message: &ResponseMessage) -> Result<NewsArticleBody, Error> {
    decode_news_article_proto(message.require_proto()?)
}

pub(in crate::news) fn decode_tick_news(message: &ResponseMessage) -> Result<NewsArticle, Error> {
    decode_tick_news_proto(message.require_proto()?)
}

pub(crate) fn decode_tick_news_proto(bytes: &[u8]) -> Result<NewsArticle, Error> {
    let p = crate::proto::TickNews::decode(bytes)?;

    let millis = p.timestamp.unwrap_or_default();
    let time =
        OffsetDateTime::from_unix_timestamp_nanos((millis as i128) * 1_000_000).map_err(|e| Error::parse_field(millis.to_string(), e.to_string()))?;

    Ok(NewsArticle {
        time,
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: p.extra_data.unwrap_or_default(),
    })
}

pub(crate) fn decode_news_bulletin_proto(bytes: &[u8]) -> Result<NewsBulletin, Error> {
    let p = crate::proto::NewsBulletin::decode(bytes)?;
    Ok(NewsBulletin {
        message_id: p.news_msg_id.unwrap_or_default(),
        message_type: p.news_msg_type.unwrap_or_default(),
        message: p.news_message.unwrap_or_default(),
        exchange: p.originating_exch.unwrap_or_default(),
    })
}

pub(crate) fn decode_news_article_proto(bytes: &[u8]) -> Result<NewsArticleBody, Error> {
    let p = crate::proto::NewsArticle::decode(bytes)?;
    Ok(NewsArticleBody {
        article_type: ArticleType::from(p.article_type.unwrap_or_default()),
        article_text: p.article_text.unwrap_or_default(),
    })
}

pub(crate) fn decode_historical_news_proto(bytes: &[u8]) -> Result<NewsArticle, Error> {
    let p = crate::proto::HistoricalNews::decode(bytes)?;

    let time = p.time.as_deref().and_then(try_parse_time_as_utc).unwrap_or(OffsetDateTime::UNIX_EPOCH);

    Ok(NewsArticle {
        time,
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: String::new(),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
