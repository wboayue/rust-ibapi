use super::*;
use crate::common::test_utils::helpers::assert_tws_error_message;

fn test_context() -> DecoderContext {
    DecoderContext::new(176, None)
}

fn error_message() -> ResponseMessage {
    ResponseMessage::from_simple("4|2|9000|10089|Requested market data is not subscribed|")
}

#[test]
fn test_news_bulletin_decode_error_message() {
    // Error on the request_id channel surfaces as Error::Notice, not silently
    // skipped via UnexpectedResponse (#434).
    let mut message = error_message();
    let err = NewsBulletin::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}

#[test]
fn test_news_article_decode_error_message() {
    let mut message = error_message();
    let err = NewsArticle::decode(&test_context(), &mut message).unwrap_err();
    assert_tws_error_message(err, 10089, "not subscribed");
}
