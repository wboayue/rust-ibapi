use crate::{
    messages::{OutgoingMessages, RequestMessage},
    Error,
};

pub(super) fn encode_request_news_providers() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsProviders);

    Ok(message)
}
