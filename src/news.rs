use std::fmt::Debug;

use crate::{domain::NewsProvider, errors::ErrorKind, Client, Error};

// https://interactivebrokers.github.io/tws-api/news.html

/// Historical News Headlines

/// Requesting News Articles

/// Requests news providers which the user has subscribed to.
pub fn news_providers(client: &Client) -> Result<Vec<NewsProvider>, Error> {
    // request = RequestNewsProvidersRequest::new()
    // packet = request.encode()
    // client.send_packet(packet)
    // packet = client.receive_packet(request_id)
    // ReceiveNewsProvidersResponse::decode(packet)
    print!("client: {client:?}");
    Err(Error::Regular(ErrorKind::NotImplemented))
}

// :reqNewsArticle below.

// reqHistoricalNews

//reqNewsArticle s
