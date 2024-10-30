use crate::{
    messages::{OutgoingMessages, RequestMessage},
    Error,
};

pub(super) fn encode_request_news_providers() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsProviders);

    Ok(message)
}

pub(super) fn encode_request_news_bulletins(all_messages: bool) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    const VERSION: i32 = 1;

    message.push_field(&OutgoingMessages::RequestNewsBulletins);
    message.push_field(&VERSION);
    message.push_field(&all_messages);

    Ok(message)
}

pub(super) fn encode_cancel_news_bulletin() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    const VERSION: i32 = 1;

    message.push_field(&OutgoingMessages::CancelNewsBulletin);
    message.push_field(&VERSION);

    Ok(message)
}
