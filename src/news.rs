use crate::{client::{ResponseContext, SharesChannel, Subscribable, Subscription}, messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage}, server_versions, Client, Error};
use serde::{Deserialize, Serialize};

mod decoders;
mod encoders;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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

impl SharesChannel for Vec<NewsProvider> {}

/// IB News Bulletin
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NewsBulletin {
    /// The unique identifier of the news bulletin.
    pub message_id: i32,
    /// The type of the news bulletin. One of: 1 - Regular news bulletin 2 - Exchange no longer available for trading 3 - Exchange is available for trading.
    pub message_type: i32,
    /// The text of the news bulletin.
    pub message: String,
    /// The exchange from which this news bulletin originated.
    pub exchange: String,
}

impl Subscribable<NewsBulletin> for NewsBulletin {
    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<NewsBulletin, Error> {
        match message.message_type() {
            IncomingMessages::NewsBulletins => Ok(decoders::decode_news_bulletin(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: &ResponseContext) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_news_bulletin()
    }
}

// Subscribes to IB's News Bulletins.
pub fn news_bulletins(client: &Client, all_messages: bool) -> Result<Subscription<NewsBulletin>, Error> {
    let request = encoders::encode_request_news_bulletins(all_messages)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestNewsBulletins, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

impl SharesChannel for Subscription<'_, NewsBulletin> {}
