use super::*;
use crate::common::test_utils::helpers::assert_rejects_text_framing;
use crate::messages::IncomingMessages;

#[test]
fn decode_news_bulletin_proto_works() {
    let proto_msg = crate::proto::NewsBulletin {
        news_msg_id: Some(42),
        news_msg_type: Some(1),
        news_message: Some("Market closed early".into()),
        originating_exch: Some("NYSE".into()),
    };

    let result = decode_news_bulletin_proto(proto_msg).unwrap();
    assert_eq!(result.message_id, 42);
    assert_eq!(result.message_type, 1);
    assert_eq!(result.message, "Market closed early");
    assert_eq!(result.exchange, "NYSE");
}

#[test]
fn decode_news_article_proto_works() {
    let proto_msg = crate::proto::NewsArticle {
        req_id: Some(1),
        article_type: Some(0),
        article_text: Some("Full article text here".into()),
    };

    let result = decode_news_article_proto(proto_msg).unwrap();
    assert_eq!(result.article_type, ArticleType::Text);
    assert_eq!(result.article_text, "Full article text here");
}

#[test]
fn decode_news_article_proto_article_types() {
    let decode = |article_type| {
        decode_news_article_proto(crate::proto::NewsArticle {
            req_id: Some(1),
            article_type,
            article_text: None,
        })
    };

    assert_eq!(decode(Some(1)).unwrap().article_type, ArticleType::Binary);
    assert_eq!(decode(None).unwrap().article_type, ArticleType::Text, "absent is code 0");
    let err = decode(Some(2)).unwrap_err();
    assert!(
        matches!(&err, Error::Parse(_, field, reason) if field == "article_type" && reason.contains('2')),
        "{err:?}"
    );
}

#[test]
fn decode_historical_news_proto_works() {
    let proto_msg = crate::proto::HistoricalNews {
        req_id: Some(1),
        time: Some("2023-04-10 13:30:00.000".into()),
        provider_code: Some("BRFG".into()),
        article_id: Some("BRFG$12345".into()),
        headline: Some("Market Update".into()),
    };

    let result = decode_historical_news_proto(proto_msg).unwrap();
    assert_eq!(result.provider_code, "BRFG");
    assert_eq!(result.article_id, "BRFG$12345");
    assert_eq!(result.headline, "Market Update");
    assert_ne!(result.time, OffsetDateTime::UNIX_EPOCH);
}

#[test]
fn decode_news_providers_proto_works() {
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
    let result = decode_news_providers_proto(proto_msg).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].code, "BRFG");
    assert_eq!(result[0].name, "Briefing.com");
    assert_eq!(result[1].code, "DJ-N");
    assert_eq!(result[1].name, "Dow Jones News");
}

#[test]
fn decode_news_providers_proto_empty() {
    let proto_msg = crate::proto::NewsProviders { news_providers: vec![] };
    let result = decode_news_providers_proto(proto_msg).unwrap();
    assert!(result.is_empty());
}

#[test]
fn decode_news_bulletin_rejects_text_framing() {
    assert_rejects_text_framing(IncomingMessages::NewsBulletins, "14\01\01\02\0msg\0NYSE\0", decode_news_bulletin);
}

#[test]
fn decode_historical_news_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::HistoricalNews,
        "86\09000\02024-12-23 19:45:00.0\0DJ-N\0a\0h\0",
        decode_historical_news,
    );
}

#[test]
fn decode_tick_news_proto_works() {
    let proto_msg = crate::proto::TickNews {
        req_id: Some(9000),
        timestamp: Some(1_672_531_200_000),
        provider_code: Some("BZ".into()),
        article_id: Some("BZ$123".into()),
        headline: Some("Breaking news headline".into()),
        extra_data: Some("TSLA:123".into()),
    };

    let result = decode_tick_news_proto(proto_msg).unwrap();
    assert_eq!(result.provider_code, "BZ");
    assert_eq!(result.article_id, "BZ$123");
    assert_eq!(result.headline, "Breaking news headline");
    assert_eq!(result.extra_data, "TSLA:123");
    assert_eq!(result.time.unix_timestamp(), 1_672_531_200);
}

#[test]
fn decode_tick_news_proto_invalid_timestamp() {
    let proto_msg = crate::proto::TickNews {
        timestamp: Some(i64::MAX),
        ..Default::default()
    };

    let err = decode_tick_news_proto(proto_msg).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(&i64::MAX.to_string()), "error should include the bad value: {msg}");
}

#[test]
fn decode_tick_news_rejects_text_framing() {
    assert_rejects_text_framing(
        IncomingMessages::TickNews,
        "84\09000\01672531200\0BZ\0BZ$123\0Breaking\0extra\0",
        decode_tick_news,
    );
}
