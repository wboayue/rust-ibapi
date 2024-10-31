use time::OffsetDateTime;

use crate::{
    messages::{OutgoingMessages, RequestMessage},
    server_versions, Error,
};

pub(super) fn encode_request_news_providers() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsProviders);

    Ok(message)
}

pub(super) fn encode_request_news_bulletins(all_messages: bool) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsBulletins);
    message.push_field(&VERSION);
    message.push_field(&all_messages);

    Ok(message)
}

pub(super) fn encode_cancel_news_bulletin() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelNewsBulletin);
    message.push_field(&VERSION);

    Ok(message)
}

pub(super) fn encode_request_historical_news(
    server_version: i32,
    request_id: i32,
    contract_id: i32,
    provider_codes: &[&str],
    start_time: OffsetDateTime,
    end_time: OffsetDateTime,
    total_results: u8,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestHistoricalNews);
    message.push_field(&request_id);
    message.push_field(&contract_id);
    message.push_field(&provider_codes.join("+"));
    message.push_field(&start_time);
    message.push_field(&end_time);
    message.push_field(&(total_results as i32));
    if server_version >= server_versions::NEWS_QUERY_ORIGINS {
        message.push_field(&"");
    }

    Ok(message)
}

pub(super) fn encode_request_news_article(
    server_version: i32,
    request_id: i32,
    provider_code: &str,
    article_id: &str,
) -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsArticle);
    message.push_field(&request_id);
    message.push_field(&provider_code);
    message.push_field(&article_id);

    if server_version >= server_versions::NEWS_QUERY_ORIGINS {
        message.push_field(&"");
    }

    Ok(message)
}
