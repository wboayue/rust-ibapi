use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::domain::NewsProvider;

// https://interactivebrokers.github.io/tws-api/news.html

/// Historical News Headlines

/// Requesting News Articles

/// Requests news providers which the user has subscribed to.
pub fn news_providers<C: Client>(client: &C) -> Result<Vec<NewsProvider>> {
    // request = RequestNewsProvidersRequest::new()
    // packet = request.encode()
    // client.send_packet(packet)
    // packet = client.receive_packet(request_id)
    // ReceiveNewsProvidersResponse::decode(packet)
    Err(anyhow!("not implemented!"))
}
