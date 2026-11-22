//! Shared fixtures for the scanner sync and async tests.

use crate::common::test_utils::helpers::{proto_error_response, proto_response, TEST_REQ_ID_FIRST};
use crate::contracts::Symbol;
use crate::messages::{IncomingMessages, Notice, NoticeCategory, ResponseMessage, HMDS_QUERY_MESSAGE_CODE, NO_SECURITY_DEFINITION_CODE};
use crate::scanner::{ScannerData, ScannerSubscription};
use crate::testdata::builders::scanner::{scanner_data, scanner_data_row};
use crate::testdata::builders::ResponseProtoEncoder;

/// The 165 texts TWS sends on a scanner that goes on: an empty query (seen live before RTH,
/// #886), and a bond scan's partial result, sent before its rows (`docs/wire-facts/scanner.md`).
pub(crate) const HMDS_QUERY_MESSAGES: [&str; 2] = [
    "Historical Market Data Service query message:no items retrieved",
    "Historical Market Data Service query message:10 out of 18957 items retrieved",
];

/// A genuine rejection: still ends the subscription.
pub(crate) const REJECTION_CODE: i32 = NO_SECURITY_DEFINITION_CODE;

const NVDA_CONTRACT_ID: i32 = 4815747;

/// Errors that must still end a scanner subscription.
pub(crate) const TERMINAL_SCANNER_ERRORS: &[(i32, &str)] = &[(309, "Maximum number of scanner subscriptions reached")];

pub(crate) fn hmds_query_parameters() -> ScannerSubscription {
    ScannerSubscription {
        location_code: Some("STK.US".into()),
        ..ScannerSubscription::new("STK", "TOP_OPEN_PERC_GAIN")
    }
}

/// Queued before the first poll: a 165 notice with `message`, a filled batch,
/// an empty batch, a genuine rejection, then a batch the ended stream must not
/// yield.
pub(crate) fn hmds_query_then_batches(message: &str) -> Vec<ResponseMessage> {
    let filled = scanner_data()
        .request_id(TEST_REQ_ID_FIRST)
        .rows(vec![scanner_data_row(0, NVDA_CONTRACT_ID, "NVDA")])
        .encode_proto();
    vec![
        proto_error_response(TEST_REQ_ID_FIRST, HMDS_QUERY_MESSAGE_CODE, message),
        proto_response(IncomingMessages::ScannerData, filled.clone()),
        empty_batch(),
        proto_error_response(TEST_REQ_ID_FIRST, REJECTION_CODE, "No security definition"),
        proto_response(IncomingMessages::ScannerData, filled),
    ]
}

fn empty_batch() -> ResponseMessage {
    proto_response(
        IncomingMessages::ScannerData,
        scanner_data().request_id(TEST_REQ_ID_FIRST).rows(vec![]).encode_proto(),
    )
}

/// An error frame followed by an empty batch the ended stream must not yield.
pub(crate) fn error_then_empty_batch(code: i32, message: &str) -> Vec<ResponseMessage> {
    vec![proto_error_response(TEST_REQ_ID_FIRST, code, message), empty_batch()]
}

pub(crate) fn assert_hmds_query_notice(notice: &Notice, message: &str) {
    assert_eq!(notice.request_id, Some(TEST_REQ_ID_FIRST));
    assert_eq!(notice.code, HMDS_QUERY_MESSAGE_CODE);
    assert_eq!(notice.message, message);
    assert_eq!(notice.category(), NoticeCategory::Warning);
    assert!(notice.is_informational());
}

pub(crate) fn assert_filled_batch(rows: &[ScannerData]) {
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rank, 0);
    assert_eq!(rows[0].contract_details.contract.contract_id, NVDA_CONTRACT_ID);
    assert_eq!(rows[0].contract_details.contract.symbol, Symbol::from("NVDA"));
}
