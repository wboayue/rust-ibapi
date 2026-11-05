use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};

use super::super::{ArticleType, NewsArticle, NewsArticleBody, NewsBulletin, NewsProvider};
use crate::messages::ResponseMessage;
use crate::Error;

// All originating outgoing-request gates for the news-domain messages
// (`PROTOBUF_MARKET_DATA` = 206 for `TickNews`, `PROTOBUF_NEWS_DATA` = 209 for
// the rest) sit at or below the connection floor, so the server always emits
// proto framing for these messages — text-framed arrival is rejected via
// `ResponseMessage::require_proto`, which raises `Error::UnexpectedWireFormat`
// (docs/rules/wire/proto-only-decoding.md).

pub(crate) fn decode_news_providers_proto(p: crate::proto::NewsProviders) -> Result<Vec<NewsProvider>, Error> {
    Ok(p.news_providers
        .into_iter()
        .map(|np| NewsProvider {
            code: np.provider_code.unwrap_or_default(),
            name: np.provider_name.unwrap_or_default(),
        })
        .collect())
}

pub(in crate::news) fn decode_news_bulletin(message: &ResponseMessage) -> Result<NewsBulletin, Error> {
    decode_news_bulletin_proto(message.proto()?)
}

pub(in crate::news) fn decode_historical_news(message: &ResponseMessage) -> Result<NewsArticle, Error> {
    decode_historical_news_proto(message.proto()?)
}

/// Parses a `HistoricalNews` `time`: zone-less UTC `YYYY-MM-DD HH:MM:SS.f`
/// (docs/wire-facts/news.md). Any other shape is `None` with a warning, not an
/// error: the reference client delivers the article regardless, and an error
/// would end the subscription. The warning makes a format change visible.
fn try_parse_time_as_utc(time: &str) -> Option<OffsetDateTime> {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    match PrimitiveDateTime::parse(time, format) {
        Ok(dt) => Some(dt.assume_utc()),
        Err(e) => {
            log::warn!("news article time {time:?} not parsed, leaving it unset: {e}");
            None
        }
    }
}

/// Converts a `TickNews` `timestamp` (epoch milliseconds). Out of range is
/// `None` with a warning, for the same reason as [`try_parse_time_as_utc`].
fn try_time_from_millis(millis: i64) -> Option<OffsetDateTime> {
    match OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000) {
        Ok(time) => Some(time),
        Err(e) => {
            log::warn!("news article timestamp {millis} out of range, leaving it unset: {e}");
            None
        }
    }
}

pub(in crate::news) fn decode_tick_news(message: &ResponseMessage) -> Result<NewsArticle, Error> {
    decode_tick_news_proto(message.proto()?)
}

pub(crate) fn decode_tick_news_proto(p: crate::proto::TickNews) -> Result<NewsArticle, Error> {
    Ok(NewsArticle {
        time: p.timestamp.and_then(try_time_from_millis),
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: p.extra_data.unwrap_or_default(),
    })
}

pub(crate) fn decode_news_bulletin_proto(p: crate::proto::NewsBulletin) -> Result<NewsBulletin, Error> {
    Ok(NewsBulletin {
        message_id: p.news_msg_id.unwrap_or_default(),
        message_type: p.news_msg_type.unwrap_or_default(),
        message: p.news_message.unwrap_or_default(),
        exchange: p.originating_exch.unwrap_or_default(),
    })
}

pub(crate) fn decode_news_article_proto(p: crate::proto::NewsArticle) -> Result<NewsArticleBody, Error> {
    Ok(NewsArticleBody {
        article_type: decode_article_type(p.article_type.unwrap_or_default())?,
        article_text: p.article_text.unwrap_or_default(),
    })
}

// One-shot path, so closed: an unknown code fails the call rather than reading as `Text`.
// Absent is `Text` (code 0), as in the reference client. Same table as `From<i32> for
// ArticleType`, minus its coercion.
fn decode_article_type(code: i32) -> Result<ArticleType, Error> {
    match code {
        0 => Ok(ArticleType::Text),
        1 => Ok(ArticleType::Binary),
        _ => Err(Error::parse_proto("article_type", format!("unknown article type code {code}"))),
    }
}

pub(crate) fn decode_historical_news_proto(p: crate::proto::HistoricalNews) -> Result<NewsArticle, Error> {
    Ok(NewsArticle {
        // Absent or empty (the reference client's default) is unset without a warning.
        time: p.time.as_deref().filter(|t| !t.is_empty()).and_then(try_parse_time_as_utc),
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: String::new(),
    })
}

#[cfg(test)]
#[path = "decoders_tests.rs"]
mod tests;
