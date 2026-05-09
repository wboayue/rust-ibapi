use super::*;
use time::macros::datetime;

#[test]
fn test_parse_unix_timestamp() {
    let result = parse_unix_timestamp("1681133400000").unwrap();
    assert_eq!(result, datetime!(2023-04-10 13:30:00 UTC));
}

#[test]
fn test_parse_unix_timestamp_invalid() {
    let err = parse_unix_timestamp("not_a_number").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not_a_number"), "error should include the bad value: {msg}");
    assert!(msg.contains("invalid digit"), "error should include parse reason: {msg}");
}

#[test]
fn test_decode_news_bulletin_proto() {
    let proto_msg = crate::proto::NewsBulletin {
        news_msg_id: Some(42),
        news_msg_type: Some(1),
        news_message: Some("Market closed early".into()),
        originating_exch: Some("NYSE".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_news_bulletin_proto(&bytes).unwrap();
    assert_eq!(result.message_id, 42);
    assert_eq!(result.message_type, 1);
    assert_eq!(result.message, "Market closed early");
    assert_eq!(result.exchange, "NYSE");
}

#[test]
fn test_decode_news_article_proto() {
    let proto_msg = crate::proto::NewsArticle {
        req_id: Some(1),
        article_type: Some(0),
        article_text: Some("Full article text here".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_news_article_proto(&bytes).unwrap();
    assert_eq!(result.article_type, ArticleType::Text);
    assert_eq!(result.article_text, "Full article text here");
}

#[test]
fn test_decode_historical_news_proto() {
    let proto_msg = crate::proto::HistoricalNews {
        req_id: Some(1),
        time: Some("2023-04-10 13:30:00.000".into()),
        provider_code: Some("BRFG".into()),
        article_id: Some("BRFG$12345".into()),
        headline: Some("Market Update".into()),
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_historical_news_proto(&bytes).unwrap();
    assert_eq!(result.provider_code, "BRFG");
    assert_eq!(result.article_id, "BRFG$12345");
    assert_eq!(result.headline, "Market Update");
    assert_ne!(result.time, OffsetDateTime::UNIX_EPOCH);
}

#[test]
fn test_decode_news_providers_proto() {
    let proto_msg = crate::proto::NewsProviders {
        news_providers: vec![
            crate::proto::NewsProvider {
                provider_code: Some("BRFG".into()),
                provider_name: Some("Briefing.com".into()),
            },
            crate::proto::NewsProvider {
                provider_code: Some("DJ-N".into()),
                provider_name: Some("Dow Jones News".into()),
            },
        ],
    };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_news_providers_proto(&bytes).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].code, "BRFG");
    assert_eq!(result[0].name, "Briefing.com");
    assert_eq!(result[1].code, "DJ-N");
    assert_eq!(result[1].name, "Dow Jones News");
}

#[test]
fn test_decode_news_providers_proto_empty() {
    let proto_msg = crate::proto::NewsProviders { news_providers: vec![] };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_news_providers_proto(&bytes).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_decode_news_providers_rejects_text_framing() {
    let message = ResponseMessage::from("newsProviders\01\0BZ\0Benzinga\0");
    let err = decode_news_providers(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}

#[test]
fn test_decode_news_bulletin_rejects_text_framing() {
    let message = ResponseMessage::from("14\01\01\02\0msg\0NYSE\0");
    let err = decode_news_bulletin(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}

#[test]
fn test_decode_historical_news_rejects_text_framing() {
    let message = ResponseMessage::from("86\09000\02024-12-23 19:45:00.0\0DJ-N\0a\0h\0");
    let err = decode_historical_news(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}

#[test]
fn test_decode_news_article_rejects_text_framing() {
    let message = ResponseMessage::from("83\09000\00\0body\0");
    let err = decode_news_article(&message).unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)), "expected UnexpectedResponse, got {err:?}");
}
