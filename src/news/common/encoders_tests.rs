use time::macros::datetime;

use super::*;
use crate::common::test_utils::helpers::TEST_CONTRACT_ID;
use crate::testdata::builders::news::{historical_news_request, news_bulletins_request};
use crate::testdata::builders::RequestEncoder;

/// Each bound goes in the one field TWS reads for it, as UTC `yyyy-MM-dd HH:mm:ss.0`
/// (docs/wire-facts/news.md); the other field is left out.
#[test]
fn encode_request_historical_news_bounds() {
    let cases = [
        ("no bound: both fields omitted", None, historical_news_request()),
        (
            "before: startDateTime",
            Some(NewsBound::Before(datetime!(2026-11-04 12:00 UTC))),
            historical_news_request().start_date_time("2026-11-04 12:00:00.0"),
        ),
        (
            "since: endDateTime",
            Some(NewsBound::Since(datetime!(2026-11-04 12:00 UTC))),
            historical_news_request().end_date_time("2026-11-04 12:00:00.0"),
        ),
        (
            "an offset time is sent as UTC",
            Some(NewsBound::Before(datetime!(2026-11-04 7:00:30 -5))),
            historical_news_request().start_date_time("2026-11-04 12:00:30.0"),
        ),
    ];
    for (label, bound, expected) in cases {
        let bytes = encode_request_historical_news(9000, TEST_CONTRACT_ID, &["DJ-N", "BRFG"], bound, 50).unwrap();
        let expected = expected
            .request_id(9000)
            .provider_codes(&["DJ-N", "BRFG"])
            .total_results(50)
            .encode_request();
        assert_eq!(bytes, expected, "{label}");
    }
}

#[test]
fn encode_request_news_bulletins_all_messages() {
    for (all_messages, sent) in [(AllMessages::Yes, true), (AllMessages::No, false)] {
        let bytes = encode_request_news_bulletins(all_messages).unwrap();
        assert_eq!(bytes, news_bulletins_request().all_messages(sent).encode_request(), "{all_messages:?}");
    }
}
