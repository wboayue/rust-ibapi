use time::OffsetDateTime;

use crate::{
    messages::{OutgoingMessages, RequestMessage},
    server_versions, Error,
};

pub(in crate::news) fn encode_request_news_providers() -> Result<RequestMessage, Error> {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsProviders);

    Ok(message)
}

pub(in crate::news) fn encode_request_news_bulletins(all_messages: bool) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestNewsBulletins);
    message.push_field(&VERSION);
    message.push_field(&all_messages);

    Ok(message)
}

pub(in crate::news) fn encode_cancel_news_bulletin() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelNewsBulletin);
    message.push_field(&VERSION);

    Ok(message)
}

pub(in crate::news) fn encode_request_historical_news(
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

pub(in crate::news) fn encode_request_news_article(
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

// === Protobuf Encoders ===

#[allow(dead_code)]
pub(in crate::news) fn encode_request_news_providers_proto() -> Result<Vec<u8>, crate::Error> {
    crate::proto::encoders::encode_empty_proto!(NewsProvidersRequest, crate::messages::OutgoingMessages::RequestNewsProviders)
}

#[allow(dead_code)]
pub(in crate::news) fn encode_request_news_bulletins_proto(all_messages: bool) -> Result<Vec<u8>, crate::Error> {
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use prost::Message;
    let request = crate::proto::NewsBulletinsRequest {
        all_messages: if all_messages { Some(true) } else { None },
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestNewsBulletins as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(in crate::news) fn encode_cancel_news_bulletin_proto() -> Result<Vec<u8>, crate::Error> {
    crate::proto::encoders::encode_empty_proto!(CancelNewsBulletins, crate::messages::OutgoingMessages::CancelNewsBulletin)
}

#[allow(dead_code)]
pub(in crate::news) fn encode_request_historical_news_proto(
    request_id: i32,
    contract_id: i32,
    provider_codes: &[&str],
    start_time: time::OffsetDateTime,
    end_time: time::OffsetDateTime,
    total_results: u8,
) -> Result<Vec<u8>, crate::Error> {
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use crate::ToField;
    use prost::Message;
    let request = crate::proto::HistoricalNewsRequest {
        req_id: Some(request_id),
        con_id: Some(contract_id),
        provider_codes: Some(provider_codes.join("+")),
        start_date_time: Some(start_time.to_field()),
        end_date_time: Some(end_time.to_field()),
        total_results: Some(total_results as i32),
        historical_news_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestHistoricalNews as i32,
        &request.encode_to_vec(),
    ))
}

#[allow(dead_code)]
pub(in crate::news) fn encode_request_news_article_proto(request_id: i32, provider_code: &str, article_id: &str) -> Result<Vec<u8>, crate::Error> {
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use prost::Message;
    let request = crate::proto::NewsArticleRequest {
        req_id: Some(request_id),
        provider_code: Some(provider_code.to_string()),
        article_id: Some(article_id.to_string()),
        news_article_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestNewsArticle as i32,
        &request.encode_to_vec(),
    ))
}

#[cfg(test)]
mod proto_tests {
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::messages::OutgoingMessages;

    #[test]
    fn test_encode_request_news_providers_proto() {
        let bytes = super::encode_request_news_providers_proto().unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestNewsProviders);
    }

    #[test]
    fn test_encode_request_news_bulletins_proto() {
        let bytes = super::encode_request_news_bulletins_proto(true).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestNewsBulletins);
    }

    #[test]
    fn test_encode_cancel_news_bulletin_proto() {
        let bytes = super::encode_cancel_news_bulletin_proto().unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelNewsBulletin);
    }

    #[test]
    fn test_encode_request_news_article_proto() {
        let bytes = super::encode_request_news_article_proto(1000, "BRFG", "BRFG$12345").unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestNewsArticle);
        use prost::Message;
        let req = crate::proto::NewsArticleRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.req_id, Some(1000));
        assert_eq!(req.provider_code.as_deref(), Some("BRFG"));
    }
}
