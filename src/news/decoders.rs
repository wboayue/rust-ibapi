use super::{Error, NewsProvider};
use crate::messages::ResponseMessage;

pub(super) fn decode_news_providers(mut message: ResponseMessage) -> Result<Vec<NewsProvider>, Error> {
    message.skip(); // message type

    let num_providers = message.next_int()?;
    let mut news_providers = Vec::with_capacity(num_providers as usize);

    for _ in 0..num_providers {
        news_providers.push(NewsProvider {
            code: message.next_string()?,
            name: message.next_string()?,
        });
    }

    Ok(news_providers)
}
