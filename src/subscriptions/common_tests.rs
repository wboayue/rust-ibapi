use super::*;
use crate::messages::ResponseMessage;

#[test]
fn test_is_stream_end() {
    let test_msg = ResponseMessage::from_simple("test");
    assert!(is_stream_end(&Error::EndOfStream));
    assert!(!is_stream_end(&Error::UnexpectedResponse(test_msg)));
    assert!(!is_stream_end(&Error::ConnectionFailed));
}

#[test]
fn test_should_store_error() {
    let test_msg = ResponseMessage::from_simple("test");
    assert!(!should_store_error(&Error::EndOfStream));
    assert!(should_store_error(&Error::UnexpectedResponse(test_msg)));
    assert!(should_store_error(&Error::ConnectionFailed));
}

#[test]
fn test_process_decode_result() {
    // Test success case
    match process_decode_result::<i32>(Ok(42)) {
        ProcessingResult::Success(val) => assert_eq!(val, 42),
        _ => panic!("Expected Success"),
    }

    // Test EndOfStream
    match process_decode_result::<i32>(Err(Error::EndOfStream)) {
        ProcessingResult::EndOfStream => {}
        _ => panic!("Expected EndOfStream"),
    }

    // Test skip case (wrong-channel message)
    let test_msg = ResponseMessage::from_simple("test");
    match process_decode_result::<i32>(Err(Error::UnexpectedResponse(test_msg))) {
        ProcessingResult::Skip => {}
        _ => panic!("Expected Skip"),
    }

    // Test error case
    match process_decode_result::<i32>(Err(Error::ConnectionFailed)) {
        ProcessingResult::Error(Error::ConnectionFailed) => {}
        _ => panic!("Expected Error"),
    }
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
