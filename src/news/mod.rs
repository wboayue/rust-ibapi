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

// Public types - always available regardless of feature flags

/// News provider information including code and name.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsProvider {
    /// The provider code identifier.
    pub code: String,
    /// The provider's display name.
    pub name: String,
}

/// IB News Bulletin
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsBulletin {
    /// The unique identifier of the news bulletin.
    pub message_id: i32,
    /// The type of the news bulletin.
    ///
    /// Valid values are:
    /// - `1` - Regular news bulletin
    /// - `2` - Exchange no longer available for trading
    /// - `3` - Exchange is available for trading
    pub message_type: i32,
    /// The text of the news bulletin.
    pub message: String,
    /// The exchange from which this news bulletin originated.
    pub exchange: String,
}

/// Returns news headlines for requested contracts.
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
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub enum ArticleType {
    /// plain text or html
    #[default]
    Text = 0,
    /// binary data / pdf
    Binary = 1,
}

impl From<i32> for ArticleType {
    fn from(value: i32) -> Self {
        match value {
            0 => ArticleType::Text,
            1 => ArticleType::Binary,
            _ => ArticleType::Text,
        }
    }
}

/// News article body containing the full article content.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct NewsArticleBody {
    /// The type of news article ([ArticleType::Text] - plain text or html, [ArticleType::Binary] - binary data / pdf)
    pub article_type: ArticleType,
    /// The body of article (if [ArticleType::Binary], the binary data is encoded using the Base64 scheme)
    pub article_text: String,
}

// Re-export API functions based on active feature
#[cfg(feature = "sync")]
pub mod blocking {
    pub(crate) use super::sync::{broad_tape_news, contract_news, historical_news, news_article, news_bulletins, news_providers};
}

#[cfg(all(feature = "sync", not(feature = "async")))]
#[allow(unused_imports)]
pub(crate) use sync::{broad_tape_news, contract_news, historical_news, news_article, news_bulletins, news_providers};

#[cfg(feature = "async")]
pub(crate) use r#async::{broad_tape_news, contract_news, historical_news, news_article, news_bulletins, news_providers};
