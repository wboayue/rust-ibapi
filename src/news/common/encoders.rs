use time::OffsetDateTime;

pub(in crate::news) fn encode_request_news_providers() -> Result<Vec<u8>, crate::Error> {
    crate::proto::encoders::encode_empty_proto!(NewsProvidersRequest, crate::messages::OutgoingMessages::RequestNewsProviders)
}

pub(in crate::news) fn encode_request_news_bulletins(all_messages: bool) -> Result<Vec<u8>, crate::Error> {
    use crate::messages::{encode_protobuf_message, OutgoingMessages};
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::NewsBulletinsRequest {
        all_messages: some_bool(all_messages),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestNewsBulletins as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::news) fn encode_cancel_news_bulletin() -> Result<Vec<u8>, crate::Error> {
    crate::proto::encoders::encode_empty_proto!(CancelNewsBulletins, crate::messages::OutgoingMessages::CancelNewsBulletin)
}

pub(in crate::news) fn encode_request_historical_news(
    request_id: i32,
    contract_id: i32,
    provider_codes: &[&str],
    start_time: OffsetDateTime,
    end_time: OffsetDateTime,
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

pub(in crate::news) fn encode_request_news_article(request_id: i32, provider_code: &str, article_id: &str) -> Result<Vec<u8>, crate::Error> {
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

// Encoder body assertions live in the migrated sync/async tests via
// `assert_request<B>(builder)`.
