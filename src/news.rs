use std::fmt::Debug;

use anyhow::{anyhow, Result};

use crate::{Client, domain::NewsProvider};

// https://interactivebrokers.github.io/tws-api/news.html

/// Historical News Headlines

/// Requesting News Articles

/// Requests news providers which the user has subscribed to.
pub fn news_providers(client: &Client) -> Result<Vec<NewsProvider>> {
    // request = RequestNewsProvidersRequest::new()
    // packet = request.encode()
    // client.send_packet(packet)
    // packet = client.receive_packet(request_id)
    // ReceiveNewsProvidersResponse::decode(packet)
    print!("client: {client:?}");
    Err(anyhow!("not implemented!"))
}

// :reqNewsArticle below.

// reqHistoricalNews

//reqNewsArticle s
