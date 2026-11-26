use super::*;
use crate::messages::ResponseMessage;

#[test]
fn is_undeclared_works() {
    // The skip decision, which used to live in an error variant.
    let ids = &[IncomingMessages::TickPrice, IncomingMessages::TickSize];

    assert!(!is_undeclared(ids, &ResponseMessage::from("1\0payload\0")));
    assert!(!is_undeclared(ids, &ResponseMessage::from("2\0payload\0")));
    assert!(is_undeclared(ids, &ResponseMessage::from("3\0payload\0")));
    // An empty declaration skips everything — which is why the trait const has
    // no default: both drivers read it straight off the decoder type.
    assert!(is_undeclared(&[], &ResponseMessage::from("1\0payload\0")));
}

#[test]
fn decoder_context_default() {
    let context = DecoderContext::default();
    assert_eq!(context.server_version, 0);
    assert!(context.request_type.is_none());
    assert!(!context.is_smart_depth);
    assert!(!context.keep_up_to_date);
}

#[test]
fn decoder_context_new() {
    let context = DecoderContext::new(176);
    assert_eq!(context.server_version, 176);
    assert!(context.request_type.is_none());
    assert!(!context.is_smart_depth);
    assert!(!context.keep_up_to_date);
}

#[test]
fn decoder_context_builder() {
    let context = DecoderContext::new(176)
        .with_request_type(crate::messages::OutgoingMessages::RequestMarketData)
        .with_smart_depth(true)
        .with_keep_up_to_date(true);

    assert_eq!(context.server_version, 176);
    assert_eq!(context.request_type, Some(crate::messages::OutgoingMessages::RequestMarketData));
    assert!(context.is_smart_depth);
    assert!(context.keep_up_to_date);
}

#[test]
fn decoder_context_clone() {
    let context = DecoderContext {
        server_version: 176,
        is_smart_depth: true,
        keep_up_to_date: false,
        request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
    };

    let cloned = context.clone();
    assert_eq!(context, cloned);
    assert_eq!(cloned.server_version, 176);
    assert!(cloned.is_smart_depth);
    assert_eq!(cloned.request_type, Some(crate::messages::OutgoingMessages::RequestMarketData));
}

#[test]
fn decoder_context_equality() {
    struct TestCase {
        name: &'static str,
        context1: DecoderContext,
        context2: DecoderContext,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "default_contexts_equal",
            context1: DecoderContext::default(),
            context2: DecoderContext::default(),
            expected: true,
        },
        TestCase {
            name: "same_values_equal",
            context1: DecoderContext {
                server_version: 176,
                is_smart_depth: true,
                keep_up_to_date: false,
                request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
            },
            context2: DecoderContext {
                server_version: 176,
                is_smart_depth: true,
                keep_up_to_date: false,
                request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
            },
            expected: true,
        },
        TestCase {
            name: "different_smart_depth",
            context1: DecoderContext {
                is_smart_depth: true,
                keep_up_to_date: false,
                ..Default::default()
            },
            context2: DecoderContext {
                is_smart_depth: false,
                keep_up_to_date: false,
                ..Default::default()
            },
            expected: false,
        },
        TestCase {
            name: "different_request_type",
            context1: DecoderContext {
                request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
                ..Default::default()
            },
            context2: DecoderContext {
                request_type: Some(crate::messages::OutgoingMessages::CancelMarketData),
                ..Default::default()
            },
            expected: false,
        },
        TestCase {
            name: "different_server_version",
            context1: DecoderContext {
                server_version: 175,
                ..Default::default()
            },
            context2: DecoderContext {
                server_version: 176,
                ..Default::default()
            },
            expected: false,
        },
    ];

    for tc in test_cases {
        assert_eq!(tc.context1 == tc.context2, tc.expected, "test case '{}' failed", tc.name);
    }
}

#[test]
fn decoder_context_debug_format() {
    let context = DecoderContext {
        server_version: 176,
        is_smart_depth: true,
        keep_up_to_date: false,
        request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
    };

    let debug_str = format!("{:?}", context);
    assert!(debug_str.contains("DecoderContext"));
    assert!(debug_str.contains("server_version"));
    assert!(debug_str.contains("is_smart_depth"));
    assert!(debug_str.contains("true"));
    assert!(debug_str.contains("request_type"));
    assert!(debug_str.contains("Some"));
}

/// The routing guard is the enforcement behind
/// `docs/rules/wire/proto-aware-accessors.md`. `FamilyCodes` is a shared-channel
/// type with no `text_request_id_field` entry, so a request_id-keyed
/// subscription could never receive it.
struct UnroutableDecoder;

impl StreamDecoder<UnroutableDecoder> for UnroutableDecoder {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::FamilyCodes];

    fn decode(_context: &DecoderContext, message: &ResponseMessage) -> Result<Self, Error> {
        Err(Error::unexpected_response(message))
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "has no `text_request_id_field` entry")]
fn debug_assert_request_id_routable_rejects_unroutable_declaration() {
    debug_assert_request_id_routable::<UnroutableDecoder, UnroutableDecoder>(Some(42));
}

#[test]
fn debug_assert_request_id_routable_skips_unkeyed_subscriptions() {
    // Order-id and shared-channel subscriptions pass `None`; their decoders
    // legitimately declare types with no request id (positions, news bulletins).
    debug_assert_request_id_routable::<UnroutableDecoder, UnroutableDecoder>(None);
}

#[test]
fn collect_stop_into_result() {
    let rows = || vec![1, 2];

    assert_eq!(CollectStop::EndMarker.into_result(rows()).unwrap(), vec![1, 2]);
    assert_eq!(CollectStop::Stopped.into_result(rows()).unwrap(), vec![1, 2]);
    assert!(matches!(CollectStop::Closed.into_result(rows()), Err(Error::UnexpectedEndOfStream)));
    assert!(matches!(CollectStop::Deadline.into_result(rows()), Err(Error::Timeout)));
    assert!(matches!(
        CollectStop::Error(Error::ConnectionReset).into_result(rows()),
        Err(Error::ConnectionReset)
    ));
}

#[test]
fn lag_as_error_fails_only_lag_notices() {
    let lag = Notice::synthesized(SUBSCRIPTION_LAG_CODE, "fell behind".into());
    assert!(matches!(
        lag_as_error::<i32>(Ok(SubscriptionItem::Notice(lag.clone()))),
        Err(Error::Notice(n)) if n == lag
    ));

    let warning = Notice::synthesized(2104, "Market data farm OK".into());
    assert!(matches!(
        lag_as_error::<i32>(Ok(SubscriptionItem::Notice(warning.clone()))),
        Ok(SubscriptionItem::Notice(n)) if n == warning
    ));
    assert!(matches!(lag_as_error(Ok(SubscriptionItem::Data(7))), Ok(SubscriptionItem::Data(7))));
    assert!(matches!(lag_as_error::<i32>(Err(Error::ConnectionReset)), Err(Error::ConnectionReset)));
}

/// Whether `T` skips its cancel after a terminal error with `code`, on a
/// request sent with `context`.
fn rejection_ends_with<T: StreamDecoder<T>>(code: i32, context: &DecoderContext) -> bool {
    T::rejection_ends_request(&Notice::synthesized(code, String::new()), context)
}

/// [`rejection_ends_with`] on a request with the default context.
fn rejection_ends<T: StreamDecoder<T>>(code: i32) -> bool {
    rejection_ends_with::<T>(code, &DecoderContext::default())
}

/// Each opt-in covers only the code verified live to end the request at TWS;
/// another terminal code still cancels, so a request TWS keeps can't leak.
#[test]
fn rejection_ends_request_only_for_verified_codes() {
    use crate::accounts::{PnL, PnLSingle};
    use crate::contracts::ContractDetails;
    use crate::market_data::historical::HistoricalDataFrame;
    use crate::market_data::realtime::{Bar, MarketDepths, TickBidAsk, TickLast, TickMidpoint, TickTypes};
    use crate::scanner::ScannerData;

    struct Case {
        decoder: &'static str,
        ends: fn(i32) -> bool,
        /// Verified live to end the request at TWS; `None` = no opt-in.
        verified: Option<i32>,
        /// Another terminal code, which must still cancel.
        other: i32,
    }
    let case = |decoder, ends, verified, other| Case {
        decoder,
        ends,
        verified,
        other,
    };
    // Wire literals, not the `notice.rs` constants: the table pins IB's codes
    // independently of the names. `other` is the code a leak would come from
    // where one is known (10197 on market data).
    let cases = [
        case("TickTypes", rejection_ends::<TickTypes>, Some(200), 10197),
        case("Bar", rejection_ends::<Bar>, Some(200), 10197),
        case("TickLast", rejection_ends::<TickLast>, Some(200), 10197),
        case("TickBidAsk", rejection_ends::<TickBidAsk>, Some(200), 10197),
        case("TickMidpoint", rejection_ends::<TickMidpoint>, Some(200), 10197),
        case("MarketDepths", rejection_ends::<MarketDepths>, Some(200), 10197),
        case(
            "HistoricalDataFrame, keep_up_to_date",
            |code| rejection_ends_with::<HistoricalDataFrame>(code, &DecoderContext::default().with_keep_up_to_date(true)),
            Some(200),
            162,
        ),
        case("PnL", rejection_ends::<PnL>, Some(321), 200),
        case("PnLSingle", rejection_ends::<PnLSingle>, Some(321), 200),
        case("Vec<ScannerData>", rejection_ends::<Vec<ScannerData>>, Some(162), 309),
        // Silent cancel live (no 366-style reply): nothing to gain, keep the safety net.
        case("ContractDetails", rejection_ends::<ContractDetails>, None, 200),
    ];
    for Case {
        decoder,
        ends,
        verified,
        other,
    } in cases
    {
        if let Some(code) = verified {
            assert!(ends(code), "{decoder}: {code} is verified to end the request");
        }
        assert!(!ends(other), "{decoder}: {other} must still cancel");
    }
    // A one-shot query (no keep_up_to_date) has nothing to continue after any error.
    for code in [162, 200, 10197, 10314] {
        assert!(rejection_ends::<HistoricalDataFrame>(code), "one-shot HistoricalDataFrame: {code}");
    }
}

#[test]
fn cancel_frame_covers_each_cancel_outcome() {
    use crate::accounts::PnL;
    use crate::contracts::OptionChain;

    let context = DecoderContext::default();
    // The type has a cancel and the id it needs.
    assert_eq!(
        cancel_frame::<PnL>(&context, Some(9000)),
        PnL::cancel_message(0, Some(9000), None).expect("encodes"),
    );
    // The encode fails (no request id): logged, nothing to send.
    assert!(cancel_frame::<PnL>(&context, None).is_none());
    // The type has no cancel (the `StreamDecoder` default).
    assert!(cancel_frame::<OptionChain>(&context, Some(9000)).is_none());
}
