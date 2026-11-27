use super::*;
use crate::common::test_utils::helpers;
use crate::market_data::historical::common::test_tables::HMDS_CONNECTED_MESSAGE;

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

/// Code 165 reports on a query TWS goes on with, in every text seen live: the
/// farm connecting before a historical request's bars (#1136), an empty scan
/// (#886), and a bond scan's partial result before its rows.
#[test]
fn notice_classifies_every_hmds_query_message_as_warning() {
    for message in [
        HMDS_CONNECTED_MESSAGE,
        "Historical Market Data Service query message:no items retrieved",
        "Historical Market Data Service query message:10 out of 18957 items retrieved",
    ] {
        let notice = helpers::test_notice(HMDS_QUERY_MESSAGE_CODE, message);
        assert!(notice.is_warning(), "{message:?}");
        assert_eq!(notice.category(), NoticeCategory::Warning, "{message:?}");
        assert!(notice.is_informational(), "{message:?}");
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

    for code in [201, 203, 355, end - 1, end, 103, 110, 461, 10244, 10311, 10317] {
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
fn notice_is_order_change_rejection() {
    for code in [104, 105, 10067, 10147, 10148, 10241, 10242] {
        let notice = helpers::test_notice(code, "");
        assert!(notice.is_order_change_rejection(), "code {code} should be an order change rejection");
        assert!(notice.is_error(), "code {code} should still end the request");
        assert_eq!(notice.log_level(), log::Level::Error, "code {code}");
    }
    for code in [103, 110, 161, 201, 202, 10149] {
        assert!(
            !helpers::test_notice(code, "").is_order_change_rejection(),
            "code {code} should not be an order change rejection"
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
                (notice.is_order_change_rejection(), NoticeCategory::OrderChangeRejection),
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
        (103, NoticeCategory::OrderRejection),            // duplicate order id: no order created
        (110, NoticeCategory::OrderRejection),            // price off the minimum tick
        (10244, NoticeCategory::OrderRejection),          // cash quantity not allowed
        (10317, NoticeCategory::OrderRejection),          // cash quantity off its minimum variation
        (461, NoticeCategory::OrderRejection),            // size not a multiple of the board lot
        (10311, NoticeCategory::OrderRejection),          // direct routing refused by a precautionary setting
        (104, NoticeCategory::OrderChangeRejection),      // modify of a finished order
        (105, NoticeCategory::OrderChangeRejection),      // modify changes side or contract
        (10067, NoticeCategory::OrderChangeRejection),    // trailing stop price changed alone
        (10241, NoticeCategory::OrderChangeRejection),    // modify of a cash-quantity order
        (10242, NoticeCategory::OrderChangeRejection),    // modify of a fractional order
        (10147, NoticeCategory::OrderChangeRejection),    // cancel of an order not found
        (10148, NoticeCategory::OrderChangeRejection),    // cancel of a finished order
        (161, NoticeCategory::Error),                     // also follows a cancel that ended the order
        (10318, NoticeCategory::Error),                   // not verified against a cancel
        (ORDER_PRESET_TIF_CODE, NoticeCategory::Warning), // 10349: TWS goes on with the order
        (ORDER_PRESET_TIF_CODE + 1, NoticeCategory::Error),
        (100, NoticeCategory::Error),
        (502, NoticeCategory::Error),
        (10000, NoticeCategory::Error),
    ];

    for &(code, expected) in cases {
        assert_eq!(helpers::test_notice(code, "").category(), expected, "code {code} miscategorised");
    }
}

fn farm_codes(status: ConnectivityStatus) -> impl Iterator<Item = i32> {
    FARM_CODES.iter().filter(move |entry| entry.2 == status).map(|entry| entry.0)
}

#[test]
fn farm_code_table() {
    // Written out independently of FARM_CODES, from IB's Message Codes table.
    let cases = [
        (2103, DataFarmKind::MarketData, ConnectivityStatus::Broken),
        (2104, DataFarmKind::MarketData, ConnectivityStatus::Ok),
        (2108, DataFarmKind::MarketData, ConnectivityStatus::Inactive),
        (2119, DataFarmKind::MarketData, ConnectivityStatus::Connecting),
        (2105, DataFarmKind::HistoricalData, ConnectivityStatus::Broken),
        (2106, DataFarmKind::HistoricalData, ConnectivityStatus::Ok),
        (2107, DataFarmKind::HistoricalData, ConnectivityStatus::Inactive),
        (2157, DataFarmKind::SecurityDefinition, ConnectivityStatus::Broken),
        (2158, DataFarmKind::SecurityDefinition, ConnectivityStatus::Ok),
    ];
    for (code, kind, status) in cases {
        assert_eq!(DataFarmKind::from_code(code), Some(kind), "code {code}");
        assert_eq!(ConnectivityStatus::from_code(code), Some(status), "code {code}");
    }
    assert_eq!(FARM_CODES.len(), cases.len(), "FARM_CODES has a code the table above lacks");

    // Non-farm codes, including warning-band neighbors and the range boundaries,
    // carry neither a kind nor a status.
    for code in [
        *WARNING_CODE_RANGE.start(),
        2102,
        2109,
        2120,
        2156,
        2159,
        *WARNING_CODE_RANGE.end(),
        500,
        202,
        1100,
    ] {
        assert_eq!(DataFarmKind::from_code(code), None, "code {code} should have no kind");
        assert_eq!(ConnectivityStatus::from_code(code), None, "code {code} should have no status");
    }
}

#[test]
fn data_farm_reads_kind_status_and_name() {
    use ConnectivityStatus::{Broken, Connecting, Inactive, Ok as FarmOk};
    use DataFarmKind::*;
    // The first nine messages are as paper TWS/Gateway sent them
    // (docs/wire-facts/protocol.md#data-farm-notices). The rest are edge cases:
    // padded, dotted, 2107 (not captured) and missing names.
    let cases = [
        (2104, "Market data farm connection is OK:usopt", MarketData, FarmOk, Some("usopt")),
        (2104, "Market data farm connection is OK:usfarm", MarketData, FarmOk, Some("usfarm")),
        (2106, "HMDS data farm connection is OK:ushmds", HistoricalData, FarmOk, Some("ushmds")),
        (
            2158,
            "Sec-def data farm connection is OK:secdefnj",
            SecurityDefinition,
            FarmOk,
            Some("secdefnj"),
        ),
        (2119, "Market data farm is connecting:cashfarm", MarketData, Connecting, Some("cashfarm")),
        (2103, "Market data farm connection is broken:usfarm", MarketData, Broken, Some("usfarm")),
        (2105, "HMDS data farm connection is broken:ushmds", HistoricalData, Broken, Some("ushmds")),
        (
            2157,
            "Sec-def data farm connection is broken:secdefnj",
            SecurityDefinition,
            Broken,
            Some("secdefnj"),
        ),
        (
            2108,
            "Market data farm connection is inactive but should be available upon demand.cashfarm",
            MarketData,
            Inactive,
            Some("cashfarm"),
        ),
        (2103, "Market data farm connection is broken: usfarm ", MarketData, Broken, Some("usfarm")),
        (
            2108,
            "Market data farm connection is inactive but should be available upon demand.usfarm.nj",
            MarketData,
            Inactive,
            Some("usfarm.nj"),
        ),
        (
            2107,
            "HMDS data farm connection is inactive but should be available upon demand.ushmds",
            HistoricalData,
            Inactive,
            Some("ushmds"),
        ),
        (2104, "Market data farm connection is OK", MarketData, FarmOk, None),
        (2105, "HMDS data farm connection is broken:", HistoricalData, Broken, None),
        (2107, "HMDS data farm connection is inactive.", HistoricalData, Inactive, None),
    ];
    for (code, message, kind, status, name) in cases {
        let notice = helpers::test_notice(code, message);
        assert_eq!(
            notice.data_farm(),
            Some(DataFarm {
                kind,
                status,
                name: name.map(str::to_owned)
            }),
            "{message:?}"
        );
        assert_eq!(notice.connectivity_status(), Some(status), "{message:?}");
    }
    assert_eq!(helpers::test_notice(500, "Not connected:usfarm").data_farm(), None);
}

#[test]
fn connectivity_status_delegates() {
    // The accessor is a thin wrapper over from_code; assert delegation without
    // re-asserting the whole table.
    let code = FARM_CODES[0].0;
    assert_eq!(helpers::test_notice(code, "").connectivity_status(), ConnectivityStatus::from_code(code));
    assert_eq!(helpers::test_notice(500, "").connectivity_status(), None);
}

#[test]
fn connectivity_status_subset_of_warning() {
    // Additive contract: every farm code is still a Warning under the existing
    // partition — connectivity_status refines, it doesn't repartition.
    for &(code, ..) in &FARM_CODES {
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
    for code in farm_codes(ConnectivityStatus::Ok)
        .chain(farm_codes(ConnectivityStatus::Inactive))
        .chain(farm_codes(ConnectivityStatus::Connecting))
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
    .chain(farm_codes(ConnectivityStatus::Broken))
    .chain(DATA_ADVISORY_CODES.iter().copied())
    {
        assert_eq!(Notice::synthesized(code, String::new()).log_level(), log::Level::Warn, "code {code}");
    }
    // 399 grades by its text: a `Warning:` line warns, anything else is a rejection.
    let order_warning = Notice::synthesized(ORDER_MESSAGE_CODE, "Order Message:\nWarning: outside RTH".into());
    assert_eq!(order_warning.log_level(), log::Level::Warn);
    assert_eq!(Notice::synthesized(ORDER_MESSAGE_CODE, "rejected".into()).log_level(), log::Level::Error);
    // 165, whatever its text, is a routine query report: info.
    for message in [HMDS_CONNECTED_MESSAGE, "no items retrieved", "10 out of 18957 items retrieved"] {
        assert_eq!(
            Notice::synthesized(HMDS_QUERY_MESSAGE_CODE, message.into()).log_level(),
            log::Level::Info,
            "{message:?}"
        );
    }
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
