use super::*;

use crate::common::test_utils::helpers::proto_response;
use crate::market_data::historical::TickLast;
use crate::messages::{IncomingMessages, Notice};
use crate::testdata::builders::market_data::{historical_tick_last, historical_ticks_last_response};
use crate::testdata::builders::ResponseProtoEncoder;

fn tick_batch_response(done: bool) -> RoutedItem {
    RoutedItem::Response(proto_response(
        IncomingMessages::HistoricalTickLast,
        historical_ticks_last_response()
            .tick(historical_tick_last(1_681_133_400, 12.00, 100, "NYSE"))
            .tick(historical_tick_last(1_681_133_401, 12.01, 200, "NYSE"))
            .done(done)
            .encode_proto(),
    ))
}

#[test]
fn classify_tick_batch_returns_batch() {
    match classify::<TickLast>(tick_batch_response(false)) {
        TickAction::Batch(ticks, done) => {
            assert_eq!(ticks.len(), 2, "both ticks decoded");
            assert_eq!(ticks[0].price, 12.00);
            assert_eq!(ticks[1].price, 12.01);
            assert!(!done, "done flag passed through from decoder");
        }
        _ => panic!("expected Batch"),
    }
}

#[test]
fn classify_tick_batch_propagates_done() {
    match classify::<TickLast>(tick_batch_response(true)) {
        TickAction::Batch(_, done) => assert!(done, "done=true must round-trip"),
        _ => panic!("expected Batch"),
    }
}

#[test]
fn classify_unexpected_message_type_returns_skip() {
    // A well-formed message of a different type → Skip, not Error.
    let other = RoutedItem::Response(proto_response(IncomingMessages::HistoricalData, vec![]));
    assert!(matches!(classify::<TickLast>(other), TickAction::Skip), "unexpected msg type skips");
}

#[test]
fn classify_decode_failure_returns_error() {
    // Right message type, garbage proto body → decode fails → Error (not a panic;
    // this is the latent `.unwrap()` the migration removed).
    let malformed = RoutedItem::Response(proto_response(IncomingMessages::HistoricalTickLast, vec![0xFF, 0xFF, 0xFF]));
    assert!(
        matches!(classify::<TickLast>(malformed), TickAction::Error(_)),
        "decode failure surfaces as Error, never panics"
    );
}

#[test]
fn classify_notice_returns_notice() {
    let notice = Notice {
        code: 2100,
        message: "some warning".into(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    match classify::<TickLast>(RoutedItem::Notice(notice.clone())) {
        TickAction::Notice(n) => {
            assert_eq!(n.code, 2100);
            assert_eq!(n.message, notice.message);
        }
        _ => panic!("expected Notice"),
    }
}

#[test]
fn classify_end_of_stream_error_returns_end_of_stream() {
    assert!(
        matches!(classify::<TickLast>(RoutedItem::Error(Error::EndOfStream)), TickAction::EndOfStream),
        "EndOfStream is a clean terminator, not an Error"
    );
}

#[test]
fn classify_other_error_returns_error() {
    match classify::<TickLast>(RoutedItem::Error(Error::Simple("boom".into()))) {
        TickAction::Error(Error::Simple(msg)) => assert_eq!(msg, "boom"),
        _ => panic!("expected Error"),
    }
}
