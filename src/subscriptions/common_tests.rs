use super::*;
use crate::messages::ResponseMessage;

#[test]
fn test_is_undeclared() {
    // The skip decision, which used to live in an error variant.
    let ids = &[IncomingMessages::TickPrice, IncomingMessages::TickSize];

    assert!(!is_undeclared(ids, &ResponseMessage::from("1\0payload\0")));
    assert!(!is_undeclared(ids, &ResponseMessage::from("2\0payload\0")));
    assert!(is_undeclared(ids, &ResponseMessage::from("3\0payload\0")));
    // An empty declaration skips everything — which is why the trait const has
    // no default and `with_decoder` takes it as a required argument.
    assert!(is_undeclared(&[], &ResponseMessage::from("1\0payload\0")));
}

#[test]
fn test_decoder_context_default() {
    let context = DecoderContext::default();
    assert_eq!(context.server_version, 0);
    assert!(context.time_zone.is_none());
    assert!(context.request_type.is_none());
    assert!(!context.is_smart_depth);
}

#[test]
fn test_decoder_context_new() {
    let context = DecoderContext::new(176, None);
    assert_eq!(context.server_version, 176);
    assert!(context.time_zone.is_none());
    assert!(context.request_type.is_none());
    assert!(!context.is_smart_depth);
}

#[test]
fn test_decoder_context_builder() {
    let context = DecoderContext::new(176, None)
        .with_request_type(crate::messages::OutgoingMessages::RequestMarketData)
        .with_smart_depth(true);

    assert_eq!(context.server_version, 176);
    assert_eq!(context.request_type, Some(crate::messages::OutgoingMessages::RequestMarketData));
    assert!(context.is_smart_depth);
}

#[test]
fn test_decoder_context_clone() {
    let context = DecoderContext {
        server_version: 176,
        time_zone: None,
        is_smart_depth: true,
        request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
    };

    let cloned = context.clone();
    assert_eq!(context, cloned);
    assert_eq!(cloned.server_version, 176);
    assert!(cloned.is_smart_depth);
    assert_eq!(cloned.request_type, Some(crate::messages::OutgoingMessages::RequestMarketData));
}

#[test]
fn test_decoder_context_equality() {
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
                time_zone: None,
                is_smart_depth: true,
                request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
            },
            context2: DecoderContext {
                server_version: 176,
                time_zone: None,
                is_smart_depth: true,
                request_type: Some(crate::messages::OutgoingMessages::RequestMarketData),
            },
            expected: true,
        },
        TestCase {
            name: "different_smart_depth",
            context1: DecoderContext {
                is_smart_depth: true,
                ..Default::default()
            },
            context2: DecoderContext {
                is_smart_depth: false,
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
fn test_decoder_context_debug_format() {
    let context = DecoderContext {
        server_version: 176,
        time_zone: None,
        is_smart_depth: true,
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

    fn decode(_context: &DecoderContext, _message: &mut ResponseMessage) -> Result<Self, Error> {
        Err(Error::NotImplemented)
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "has no `text_request_id_field` entry")]
fn test_debug_assert_request_id_routable_rejects_unroutable_declaration() {
    debug_assert_request_id_routable::<UnroutableDecoder, UnroutableDecoder>(Some(42));
}

#[test]
fn test_debug_assert_request_id_routable_skips_unkeyed_subscriptions() {
    // Order-id and shared-channel subscriptions pass `None`; their decoders
    // legitimately declare types with no request id (positions, news bulletins).
    debug_assert_request_id_routable::<UnroutableDecoder, UnroutableDecoder>(None);
}
