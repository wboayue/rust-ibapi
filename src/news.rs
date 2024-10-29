use crate::{messages::OutgoingMessages, server_versions, Client, Error};

mod decoders;
mod encoders;

#[derive(Clone, Debug)]
pub struct NewsProvider {
    pub code: String,
    pub name: String,
}

// https://interactivebrokers.github.io/tws-api/news.html

/// Historical News Headlines

/// Requesting News Articles

/// Requests news providers which the user has subscribed to.
pub fn news_providers(client: &Client) -> Result<Vec<NewsProvider>, Error> {
    client.check_server_version(server_versions::REQ_NEWS_PROVIDERS, "It does not support news providers requests.")?;

    let request = encoders::encode_request_news_providers()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestNewsProviders, request)?;

    match subscription.next() {
        Some(Ok(message)) => decoders::decode_news_providers(message),
        Some(Err(Error::ConnectionReset)) => news_providers(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}
