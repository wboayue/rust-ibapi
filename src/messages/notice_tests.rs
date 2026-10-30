use super::*;
use crate::common::test_utils::helpers;

#[test]
fn notice() {
    let message = helpers::proto_error_response(-1, 2107, "HMDS data farm connection is inactive.");

    let notice = Notice::from(&message);

    assert_eq!(notice.code, 2107);
    assert_eq!(notice.message, "HMDS data farm connection is inactive.");
    assert_eq!(format!("{notice}"), "[2107] HMDS data farm connection is inactive.");
}

#[test]
fn notice_edge_cases() {
    struct TestCase {
        name: &'static str,
        code: i32,
        msg: &'static str,
    }

    let test_cases = vec![
        TestCase {
            name: "normal_error",
            code: 2107,
            msg: "HMDS data farm connection is inactive.",
        },
        TestCase {
            name: "empty_message",
            code: 1000,
            msg: "",
        },
        TestCase {
            name: "negative_code",
            code: -500,
            msg: "Negative error code",
        },
    ];

    for test_case in test_cases {
        let message = helpers::proto_error_response(-1, test_case.code, test_case.msg);
        let notice = Notice::from(&message);

        assert_eq!(notice.code, test_case.code, "Test '{}' failed: wrong error code", test_case.name);
        assert_eq!(notice.message, test_case.msg, "Test '{}' failed: wrong error message", test_case.name);
    }
}

#[test]
fn notice_is_cancellation() {
    // Code 202 = order cancelled
    let cancellation = Notice {
        request_id: None,
        code: 202,
        message: "Order Cancelled - reason:".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(cancellation.is_cancellation());
    assert!(cancellation.is_informational());
    assert!(!cancellation.is_error());

    // Other codes are not cancellations
    let error = Notice {
        request_id: None,
        code: 200,
        message: "No security definition found".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(!error.is_cancellation());
}

#[test]
fn notice_is_warning() {
    // Boundaries from the constant; 2176 and 2187 are the post-2169 codes IB
    // shipped that motivated widening the band (#805).
    let warning_codes = [*WARNING_CODE_RANGE.start(), 2107, 2119, 2150, 2176, 2187, *WARNING_CODE_RANGE.end()];
    for code in warning_codes {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_warning(), "Code {} should be a warning", code);
        assert!(notice.is_informational());
        assert!(!notice.is_error());
    }

    // Code 0 — a frame whose error_code field was absent on the wire — is a
    // warning even without a "Warning:" line (the undecodable-frame fallback
    // has an empty message).
    for msg in ["Warning: Approaching max rate of 50 messages per second (42)", ""] {
        let notice = Notice {
            request_id: None,
            code: 0,
            message: msg.to_string(),
            error_time: None,
            advanced_order_reject_json: String::new(),
        };
        assert!(notice.is_warning(), "code 0 with message {msg:?} should be a warning");
        assert!(notice.is_informational());
        assert!(!notice.is_error());
    }

    // Codes outside WARNING_CODE_RANGE are not warnings, nor is the advisory
    // 2188 inside it.
    let non_warning_codes = [*WARNING_CODE_RANGE.start() - 1, *WARNING_CODE_RANGE.end() + 1, 200, 202, 1000, 2188];
    for code in non_warning_codes {
        let notice = helpers::test_notice(code, "");
        assert!(!notice.is_warning(), "Code {} should not be a warning", code);
    }
}

#[test]
fn notice_classifies_order_message_warning_from_text() {
    let warning = Notice {
        request_id: Some(402000003),
        code: 399,
        message: "Order Message:\nSELL 1 ES DEC'26\nWarning: Your order will not be placed at the exchange until 2026-08-17 08:30:00 US/Central."
            .to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    let rejection = Notice {
        message: "Order Message:\nOrder cannot be transmitted".to_string(),
        ..warning.clone()
    };

    assert!(warning.is_warning());
    assert_eq!(warning.category(), NoticeCategory::Warning);
    assert!(!warning.is_error());
    assert!(!rejection.is_warning());
    assert_eq!(rejection.category(), NoticeCategory::OrderRejection);
    assert!(rejection.is_error());
}

/// TWS binds 165 "HMDS server connection was successful" to a historical
/// request when the farm connects on demand; the data follows (#1136). Other
/// 165 texts stay errors.
#[test]
fn notice_classifies_hmds_connected_as_warning() {
    let connected = helpers::test_notice(
        HMDS_QUERY_MESSAGE_CODE,
        "Historical Market Data Service query message:HMDS server connection was successful.",
    );
    assert!(connected.is_warning());
    assert_eq!(connected.category(), NoticeCategory::Warning);
    assert!(connected.is_informational());

    for message in [
        "Historical Market Data Service query message:no items retrieved",
        "Different query failure",
    ] {
        let notice = helpers::test_notice(HMDS_QUERY_MESSAGE_CODE, message);
        assert!(!notice.is_warning(), "{message:?}");
        assert_eq!(notice.category(), NoticeCategory::Error, "{message:?}");
    }
}

#[test]
fn notice_is_system_message() {
    // System message codes: 1100, 1101, 1102, 1300
    let system_codes = [
        (1100, "Connectivity between IB and TWS has been lost."),
        (1101, "Connectivity restored, data lost."),
        (1102, "Connectivity restored, data maintained."),
        (1300, "Socket port has been reset."),
    ];
    for (code, msg) in system_codes {
        let notice = Notice {
            request_id: None,
            code,
            message: msg.to_string(),
            error_time: None,
            advanced_order_reject_json: String::new(),
        };
        assert!(notice.is_system_message(), "Code {} should be a system message", code);
        assert!(notice.is_informational());
        assert!(!notice.is_error());
    }

    // Non-system codes
    let non_system_codes = [200, 202, 1099, 1103, 1299, 1301, 2100];
    for code in non_system_codes {
        let notice = Notice {
            request_id: None,
            code,
            message: format!("Non-system message with code {}", code),
            error_time: None,
            advanced_order_reject_json: String::new(),
        };
        assert!(!notice.is_system_message(), "Code {} should not be a system message", code);
    }
}

#[test]
fn notice_is_informational() {
    // Informational includes cancellations, warnings, and system messages
    let informational_codes = [202, 1100, 1101, 1102, 1300, *WARNING_CODE_RANGE.start(), 2107, *WARNING_CODE_RANGE.end()];
    for code in informational_codes {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_informational(), "Code {} should be informational", code);
        assert!(!notice.is_error(), "Code {} should not be an error", code);
    }

    // Non-informational (actual errors)
    let error_codes = [100, 200, 201, 316, 321, 354, 502, 10000];
    for code in error_codes {
        let notice = helpers::test_notice(code, "");
        assert!(!notice.is_informational(), "Code {} should not be informational", code);
        assert!(notice.is_error(), "Code {} should be an error", code);
    }
}

#[test]
fn notice_is_error() {
    // Code 200 = actual error
    let error = Notice {
        request_id: None,
        code: 200,
        message: "No security definition found".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(error.is_error());
    assert!(!error.is_informational());

    // Code 202 = cancellation, not error
    let cancellation = Notice {
        request_id: None,
        code: 202,
        message: "Order Cancelled".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(!cancellation.is_error());
    assert!(cancellation.is_informational());

    // Code 1100 = system message, not error
    let system_msg = Notice {
        request_id: None,
        code: 1100,
        message: "Connectivity lost".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(!system_msg.is_error());
    assert!(system_msg.is_informational());

    // Code 2107 = warning, not error
    let warning = Notice {
        request_id: None,
        code: 2107,
        message: "HMDS data farm connection is inactive.".to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    };
    assert!(!warning.is_error());
    assert!(warning.is_informational());
}

#[test]
fn notice_is_order_rejection() {
    let start = *ORDER_REJECTION_CODE_RANGE.start();
    let end = *ORDER_REJECTION_CODE_RANGE.end();

    for code in [201, 203, 355, end - 1, end] {
        assert!(
            helpers::test_notice(code, "").is_order_rejection(),
            "code {code} should be order rejection"
        );
    }

    // Codes inside the band that an earlier category claims are covered by
    // notice_category_predicates_are_disjoint.
    for code in [start - 1, end + 1, 100, *WARNING_CODE_RANGE.start(), SYSTEM_MESSAGE_CODES[0], 10000] {
        assert!(
            !helpers::test_notice(code, "").is_order_rejection(),
            "code {code} should not be order rejection"
        );
    }
}

#[test]
fn notice_is_request_error() {
    for &code in REQUEST_ERROR_CODES {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_request_error(), "code {code} should be a request error");
        assert!(
            ORDER_REJECTION_CODE_RANGE.contains(&code),
            "code {code} outside the band it is carved from"
        );
        assert!(notice.is_error(), "code {code} should be terminal");
    }
    for code in [201, 202, 317, 355, 399, 502, 2104, 10000] {
        assert!(
            !helpers::test_notice(code, "").is_request_error(),
            "code {code} should not be a request error"
        );
    }
}

/// Every category predicate is `category() == X`: at most one is true, and it
/// names the category. `Error` has no predicate of its own.
#[test]
fn notice_category_predicates_are_disjoint() {
    let messages = ["", "Order Message:\nWarning: outside RTH"];
    for code in -10..=11000 {
        for message in messages {
            let notice = Notice::synthesized(code, message.to_string());
            let hits: Vec<NoticeCategory> = [
                (notice.is_cancellation(), NoticeCategory::Cancellation),
                (notice.is_data_advisory(), NoticeCategory::DataAdvisory),
                (notice.is_warning(), NoticeCategory::Warning),
                (notice.is_system_message(), NoticeCategory::SystemMessage),
                (notice.is_request_error(), NoticeCategory::RequestError),
                (notice.is_order_rejection(), NoticeCategory::OrderRejection),
            ]
            .into_iter()
            .filter_map(|(hit, category)| hit.then_some(category))
            .collect();
            let expected: Vec<NoticeCategory> = match notice.category() {
                NoticeCategory::Error => vec![],
                category => vec![category],
            };
            assert_eq!(hits, expected, "code {code} message {message:?}");
        }
    }
}

#[test]
fn notice_category_partition() {
    let cases: &[(i32, NoticeCategory)] = &[
        (ORDER_CANCELLED_CODE, NoticeCategory::Cancellation), // 202 — precedence over OrderRejection
        (0, NoticeCategory::Warning),                         // code-less frame: error_code absent on the wire
        (*WARNING_CODE_RANGE.start(), NoticeCategory::Warning),
        (*WARNING_CODE_RANGE.end(), NoticeCategory::Warning),
        (SYSTEM_MESSAGE_CODES[0], NoticeCategory::SystemMessage),
        (SYSTEM_MESSAGE_CODES[3], NoticeCategory::SystemMessage),
        (*WARNING_CODE_RANGE.end() + 1, NoticeCategory::Error),
        (*ORDER_REJECTION_CODE_RANGE.start(), NoticeCategory::OrderRejection), // 200 — also answers orders
        (201, NoticeCategory::OrderRejection),                                 // hard rejection
        (316, NoticeCategory::RequestError),                                   // depth HALTED
        (320, NoticeCategory::OrderRejection),                                 // answers invalid attached orders (#842)
        (321, NoticeCategory::OrderRejection),                                 // server error validating a request
        (354, NoticeCategory::RequestError),                                   // market data not subscribed
        (366, NoticeCategory::RequestError),                                   // no historical query
        (355, NoticeCategory::OrderRejection),                                 // order size vs market rule
        (*ORDER_REJECTION_CODE_RANGE.end(), NoticeCategory::OrderRejection),   // 399
        (317, NoticeCategory::DataAdvisory),                                   // precedence over the 200..=399 band (#806)
        (2188, NoticeCategory::DataAdvisory),                                  // precedence over the 21xx band
        (10089, NoticeCategory::DataAdvisory),
        (10090, NoticeCategory::DataAdvisory),
        (10091, NoticeCategory::DataAdvisory),
        (10167, NoticeCategory::DataAdvisory),
        (100, NoticeCategory::Error),
        (502, NoticeCategory::Error),
        (10000, NoticeCategory::Error),
    ];

    for &(code, expected) in cases {
        assert_eq!(helpers::test_notice(code, "").category(), expected, "code {code} miscategorised");
    }
}

#[test]
fn connectivity_status_from_code_table() {
    // Derive expectations from the code constants, per
    // docs/rules/testing/derive-from-constants.md: each farm-code set maps to its
    // status; everything else maps to None.
    let cases: &[(&[i32], ConnectivityStatus)] = &[
        (&FARM_OK_CODES, ConnectivityStatus::Ok),
        (&FARM_BROKEN_CODES, ConnectivityStatus::Broken),
        (&FARM_INACTIVE_CODES, ConnectivityStatus::Inactive),
        (&FARM_CONNECTING_CODES, ConnectivityStatus::Connecting),
    ];
    for &(codes, expected) in cases {
        for &code in codes {
            assert_eq!(ConnectivityStatus::from_code(code), Some(expected), "code {code} misclassified");
        }
    }

    // Non-farm codes — including warning-band neighbors and the range boundaries —
    // carry no connectivity status.
    for code in [*WARNING_CODE_RANGE.start(), 2120, *WARNING_CODE_RANGE.end(), 500, 202, 1100] {
        assert_eq!(ConnectivityStatus::from_code(code), None, "code {code} should have no status");
    }
}

#[test]
fn connectivity_status_delegates() {
    // The accessor is a thin wrapper over from_code; assert delegation without
    // re-asserting the whole table.
    let code = FARM_BROKEN_CODES[0];
    assert_eq!(helpers::test_notice(code, "").connectivity_status(), ConnectivityStatus::from_code(code));
    assert_eq!(helpers::test_notice(500, "").connectivity_status(), None);
}

#[test]
fn connectivity_status_subset_of_warning() {
    // Additive contract: every farm code is still a Warning under the existing
    // partition — connectivity_status refines, it doesn't repartition.
    let all_farm = FARM_OK_CODES
        .iter()
        .chain(&FARM_BROKEN_CODES)
        .chain(&FARM_INACTIVE_CODES)
        .chain(&FARM_CONNECTING_CODES);
    for &code in all_farm {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_warning(), "farm code {code} should be a warning");
        assert_eq!(
            notice.category(),
            NoticeCategory::Warning,
            "farm code {code} should categorise as Warning"
        );
        assert!(WARNING_CODE_RANGE.contains(&code), "farm code {code} should be inside WARNING_CODE_RANGE");
    }
}

#[test]
fn notice_data_advisory() {
    // Data advisories are informational: TWS proceeds with the request and
    // data follows, so they must not be classified as errors. Exact lists win
    // over ranges: every advisory categorises as DataAdvisory whatever band it
    // is numerically inside (317 in 200..=399, 2188 in the 21xx band).
    for &code in DATA_ADVISORY_CODES {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_data_advisory(), "code {code} should be a data advisory");
        assert!(notice.is_informational(), "code {code} should be informational");
        assert!(!notice.is_error(), "code {code} should not be an error");
        assert_eq!(notice.category(), NoticeCategory::DataAdvisory, "code {code} miscategorised");

        // Adding an advisory must not classify a whole band.
        for neighbor in [code - 1, code + 1] {
            if DATA_ADVISORY_CODES.contains(&neighbor) {
                continue;
            }
            assert!(
                !helpers::test_notice(neighbor, "").is_data_advisory(),
                "code {neighbor} should not be a data advisory"
            );
        }
    }
}

#[test]
fn handshake_synthetic_constants_pinned() {
    // Hard pin the wire values — these are part of the public API and may not
    // shift silently. Negative codes never collide with TWS-emitted codes (TWS
    // uses 0+); -2 is taken by `ResponseMessage::is_shutdown`.
    assert_eq!(HANDSHAKE_UNKNOWN_FRAME_CODE, -3);
    assert_eq!(HANDSHAKE_DECODE_FAILURE_CODE, -4);
    assert_ne!(HANDSHAKE_UNKNOWN_FRAME_CODE, HANDSHAKE_DECODE_FAILURE_CODE);
}

#[test]
fn is_handshake_synthetic() {
    assert!(helpers::test_notice(HANDSHAKE_UNKNOWN_FRAME_CODE, "").is_handshake_synthetic());
    assert!(helpers::test_notice(HANDSHAKE_DECODE_FAILURE_CODE, "").is_handshake_synthetic());

    // TWS-emitted codes must not pass the predicate.
    for code in [
        0,
        ORDER_CANCELLED_CODE,
        *WARNING_CODE_RANGE.start(),
        *WARNING_CODE_RANGE.end(),
        SYSTEM_MESSAGE_CODES[0],
        *ORDER_REJECTION_CODE_RANGE.start(),
        *ORDER_REJECTION_CODE_RANGE.end(),
        -2, // shutdown sentinel — distinct sentinel, must not be confused with handshake-synthetic
        -1,
        UNKNOWN_MESSAGE_TYPE_CODE, // synthesized, but not handshake-specific
        100,
    ] {
        assert!(
            !helpers::test_notice(code, "").is_handshake_synthetic(),
            "code {code} should not be flagged handshake-synthetic"
        );
    }
}

#[test]
fn is_client_synthesized() {
    for code in [
        HANDSHAKE_UNKNOWN_FRAME_CODE,
        HANDSHAKE_DECODE_FAILURE_CODE,
        UNKNOWN_MESSAGE_TYPE_CODE,
        SUBSCRIPTION_LAG_CODE,
        NOTICE_STREAM_LAG_CODE,
        TRANSPORT_RECONNECT_CODE,
        // Any negative code, named or not.
        -1,
        -2,
        i32::MIN,
    ] {
        assert!(helpers::test_notice(code, "").is_client_synthesized(), "code {code}");
    }

    // TWS-emitted codes, including the code-less 0.
    for code in [
        0,
        ORDER_CANCELLED_CODE,
        *WARNING_CODE_RANGE.start(),
        *WARNING_CODE_RANGE.end(),
        SYSTEM_MESSAGE_CODES[0],
        *ORDER_REJECTION_CODE_RANGE.start(),
        10000,
    ] {
        assert!(!helpers::test_notice(code, "").is_client_synthesized(), "code {code}");
    }
}

#[test]
fn log_level_follows_category() {
    // Farm OK/inactive/connecting, 1102 (restored, data maintained) and the
    // cancellation confirmation: info.
    for code in FARM_OK_CODES
        .into_iter()
        .chain(FARM_INACTIVE_CODES)
        .chain(FARM_CONNECTING_CODES)
        .chain([CONNECTIVITY_RESTORED_DATA_MAINTAINED_CODE, ORDER_CANCELLED_CODE])
    {
        assert_eq!(Notice::synthesized(code, String::new()).log_level(), log::Level::Info, "code {code}");
    }
    // Broken farms, the rest of the warning band, code-less frames (0), every
    // data advisory, and 1101: warn. The advisories are the point - 317 and the
    // 10xxx codes used to log at error while 2188 logged at warn, the same
    // category twice.
    for code in [
        0,
        *WARNING_CODE_RANGE.start(),
        *WARNING_CODE_RANGE.end(),
        CONNECTIVITY_RESTORED_DATA_LOST_CODE,
    ]
    .into_iter()
    .chain(FARM_BROKEN_CODES)
    .chain(DATA_ADVISORY_CODES.iter().copied())
    {
        assert_eq!(Notice::synthesized(code, String::new()).log_level(), log::Level::Warn, "code {code}");
    }
    // 399 grades by its text: a `Warning:` line warns, anything else is a rejection.
    let order_warning = Notice::synthesized(ORDER_MESSAGE_CODE, "Order Message:\nWarning: outside RTH".into());
    assert_eq!(order_warning.log_level(), log::Level::Warn);
    assert_eq!(Notice::synthesized(ORDER_MESSAGE_CODE, "rejected".into()).log_level(), log::Level::Error);
    // Connectivity lost, socket reset, request errors (316, 354), order
    // rejections (200, 201) and errors: error.
    for code in [
        CONNECTIVITY_LOST_CODE,
        SOCKET_PORT_RESET_CODE,
        200,
        201,
        316,
        354,
        *WARNING_CODE_RANGE.end() + 1,
        10000,
    ] {
        assert_eq!(Notice::synthesized(code, String::new()).log_level(), log::Level::Error, "code {code}");
    }
}

#[test]
fn unknown_message_type_notice_names_the_id_or_its_absence() {
    let notice = unknown_message_type_notice(&ResponseMessage::from("9999\0"));
    assert_eq!(notice.code, UNKNOWN_MESSAGE_TYPE_CODE);
    assert!(notice.message.contains("message id 9999"), "{:?}", notice.message);

    // Unreachable off the wire (parse_raw_message always yields an id), but
    // the notice must not invent one.
    let notice = unknown_message_type_notice(&ResponseMessage::default());
    assert_eq!(notice.code, UNKNOWN_MESSAGE_TYPE_CODE);
    assert!(notice.message.contains("no message id"), "{:?}", notice.message);
}
