//! News data retrieval and management functionality.
//!
//! This module provides access to news articles, bulletins, and news providers
//! through the Interactive Brokers API. It supports real-time news feeds,
//! historical news queries, and news article retrieval.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// Common implementation modules
mod common;

// Feature-specific implementations
#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "async")]
mod r#async;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

// Public types - always available regardless of feature flags

/// News provider information including code and name.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsProvider {
    /// The provider code identifier.
    pub code: String,
    /// The provider's display name.
    pub name: String,
}

/// IB News Bulletin
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsBulletin {
    /// The unique identifier of the news bulletin.
    pub message_id: i32,
    /// The type of the news bulletin: a regular bulletin, or a notice that an exchange is
    /// unavailable or available for trading.
    pub message_type: BulletinType,
    /// The text of the news bulletin.
    pub message: String,
    /// The exchange from which this news bulletin originated.
    pub exchange: String,
}

/// The type of a [`NewsBulletin`].
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BulletinType {
    /// Regular news bulletin. Wire code `1`.
    Regular,
    /// Exchange no longer available for trading. Wire code `2`.
    ExchangeUnavailable,
    /// Exchange available for trading. Wire code `3`.
    ExchangeAvailable,
    /// Bulletin type code not modeled by this version of the API.
    Unknown(i32),
}

impl From<BulletinType> for i32 {
    fn from(value: BulletinType) -> i32 {
        match value {
            BulletinType::Regular => 1,
            BulletinType::ExchangeUnavailable => 2,
            BulletinType::ExchangeAvailable => 3,
            BulletinType::Unknown(code) => code,
        }
    }
}

impl From<i32> for BulletinType {
    fn from(value: i32) -> Self {
        match value {
            1 => BulletinType::Regular,
            2 => BulletinType::ExchangeUnavailable,
            3 => BulletinType::ExchangeAvailable,
            code => BulletinType::Unknown(code),
        }
    }
}

/// Returns news headlines for requested contracts.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsArticle {
    /// The article's published time.
    pub time: OffsetDateTime,
    /// The provider code for the news article.
    pub provider_code: String,
    /// Identifier used to track the particular article.
    pub article_id: String,
    /// Headline of the provided news article.
    pub headline: String,
    /// Returns any additional data available about the article.
    pub extra_data: String,
}

/// The type of news article ([ArticleType::Text] - plain text or html, [ArticleType::Binary] - binary data / pdf)
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub enum ArticleType {
    /// plain text or html
    #[default]
    Text = 0,
    /// binary data / pdf
    Binary = 1,
}

/// News article body containing the full article content.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct NewsArticleBody {
    /// The type of news article ([ArticleType::Text] - plain text or html, [ArticleType::Binary] - binary data / pdf)
    pub article_type: ArticleType,
    /// The body of article (if [ArticleType::Binary], the binary data is encoded using the Base64 scheme)
    pub article_text: String,
}
