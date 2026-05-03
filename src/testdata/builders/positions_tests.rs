//! Builder tests focused on text wire-format invariants.
//!
//! Standalone proto round-trip tests are intentionally absent: they amount to
//! `builder → prost → assert` and don't exercise production code. End-to-end
//! coverage of `builder → production decoder → domain object` lives in
//! `crate::accounts::common::decoders::tests`. End-to-end request coverage
//! (client API → captured bytes → `assert_request<B>`) lives in
//! `crate::accounts::{sync,async}::tests`.

use super::*;
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_CONTRACT_ID, TEST_TICKER_ID};
use crate::testdata::builders::ResponseEncoder;

fn split_pipe(s: &str) -> Vec<&str> {
    s.split_terminator('|').collect()
}

#[test]
fn position_default_serializes_all_fields_in_order() {
    let encoded = position().encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[0], "61");
    assert_eq!(fields[1], "3");
    assert_eq!(fields[2], TEST_ACCOUNT);
    assert_eq!(fields[3], TEST_CONTRACT_ID.to_string());
    assert_eq!(fields[4], "TSLA");
    assert_eq!(fields[5], "STK");
    assert_eq!(fields[6], "");
    assert_eq!(fields[7], "0");
    assert_eq!(fields[8], "");
    assert_eq!(fields[9], "");
    assert_eq!(fields[10], "NASDAQ");
    assert_eq!(fields[11], "USD");
    assert_eq!(fields[12], "TSLA");
    assert_eq!(fields[13], "NMS");
    assert_eq!(fields[14], "500");
    assert_eq!(fields[15], "196.77");
    assert_eq!(fields.len(), 16);
}

#[test]
fn position_setters_override_defaults() {
    let encoded = position()
        .symbol("AAPL")
        .contract_id(265598)
        .position(100.0)
        .average_cost(150.25)
        .encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[3], "265598");
    assert_eq!(fields[4], "AAPL");
    assert_eq!(fields[14], "100");
    assert_eq!(fields[15], "150.25");
}

#[test]
fn position_encode_null_uses_nul_separator() {
    let nul = position().encode_null();
    assert!(nul.starts_with("61\03\0"));
    assert!(nul.ends_with('\0'));
    assert!(!nul.contains('|'));
}

#[test]
fn position_encode_length_prefixed_wraps_null_payload() {
    let bytes = position().encode_length_prefixed();
    let payload_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    assert_eq!(payload_len, bytes.len() - 4);
    assert_eq!(&bytes[4..], position().encode_null().as_bytes());
}

#[test]
fn position_end_default_emits_header_only() {
    assert_eq!(position_end().encode_pipe(), "62|1|");
}

#[test]
fn position_multi_default_serializes_all_fields_in_order() {
    let encoded = position_multi().encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[0], "71");
    assert_eq!(fields[1], "3");
    assert_eq!(fields[2], TEST_TICKER_ID.to_string());
    assert_eq!(fields[3], TEST_ACCOUNT);
    assert_eq!(fields[4], TEST_CONTRACT_ID.to_string());
    assert_eq!(fields[5], "TSLA");
    assert_eq!(fields[6], "STK");
    assert_eq!(fields[13], "TSLA");
    assert_eq!(fields[14], "NMS");
    assert_eq!(fields[15], "500");
    assert_eq!(fields[16], "196.77");
    assert_eq!(fields[17], "");
    assert_eq!(fields.len(), 18);
}

#[test]
fn position_multi_setters_override_defaults() {
    let encoded = position_multi().request_id(42).symbol("MSFT").model_code("TARGET2024").encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[2], "42");
    assert_eq!(fields[5], "MSFT");
    assert_eq!(fields[17], "TARGET2024");
}

#[test]
fn position_multi_end_default_uses_test_ticker_id() {
    assert_eq!(position_multi_end().encode_pipe(), format!("72|1|{}|", TEST_TICKER_ID));
}

#[test]
fn position_multi_end_request_id_setter() {
    assert_eq!(position_multi_end().request_id(7).encode_pipe(), "72|1|7|");
}
