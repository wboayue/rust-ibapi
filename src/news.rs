use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::domain::NewsProvider;

/// Requests news providers which the user has subscribed to.
pub fn news_providers(client: &Client) -> Result<Vec<NewsProvider>> {
    Err(anyhow!("not implemented!"))
}
