//! Builder tests focused on text wire-format invariants.
//!
//! Standalone proto round-trip tests are intentionally absent: they amount to
//! `builder → prost → assert` and don't exercise production code. End-to-end
//! coverage of `builder → production decoder → domain object` lives in
//! `crate::orders::common::decoders::tests`. End-to-end request coverage
//! (client API → captured bytes → `assert_request<B>`) lives in
//! `crate::orders::{sync,async}::tests`.

use super::*;
use crate::testdata::builders::ResponseEncoder;

fn split_pipe(s: &str) -> Vec<&str> {
    s.split_terminator('|').collect()
}

#[test]
fn order_status_default_field_order() {
    let encoded = order_status().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "3");
    assert_eq!(fields[1], "13");
    assert_eq!(fields[2], "Submitted");
    assert_eq!(fields[3], "0");
    assert_eq!(fields[4], "100");
    assert_eq!(fields[5], "0");
    assert_eq!(fields[6], "1376327563");
    assert_eq!(fields[7], "0");
    assert_eq!(fields[8], "0");
    assert_eq!(fields[9], "100");
    assert_eq!(fields[10], "");
    assert_eq!(fields[11], "0");
}

#[test]
fn order_status_optional_doubles_become_empty_when_none() {
    let encoded = order_status()
        .average_fill_price(None)
        .last_fill_price(None)
        .market_cap_price(None)
        .encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[5], "");
    assert_eq!(fields[8], "");
    assert_eq!(fields[11], "");
}

#[test]
fn commission_report_default_field_order() {
    let encoded = commission_report().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "59");
    assert_eq!(fields[1], "1");
    assert_eq!(fields[2], "00025b46.63f8f39c.01.01");
    assert_eq!(fields[3], "1");
    assert_eq!(fields[4], "USD");
    assert_eq!(fields[5], "");
    assert_eq!(fields[6], "");
}

#[test]
fn execution_data_default_omits_version_for_post_last_liquidity() {
    let encoded = execution_data().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "11");
    assert_eq!(fields[1], "-1", "request_id slot (no version field for SIZE_RULES+)");
    assert_eq!(fields[2], "13", "order_id");
    assert_eq!(fields[3], "76792991", "contract_id");
    assert_eq!(fields[14], "00025b46.63f8f39c.01.01", "execution_id");
    assert_eq!(fields[30], "2", "last_liquidity (final field for SIZE_RULES, pre-PENDING_PRICE_REVISION)");
}

#[test]
fn open_order_end_default() {
    assert_eq!(open_order_end().encode_pipe(), "53|1|");
}

#[test]
fn execution_data_end_default() {
    assert_eq!(execution_data_end().encode_pipe(), "55|1|9000|");
}

#[test]
fn completed_orders_end_default() {
    assert_eq!(completed_orders_end().encode_pipe(), "102|");
}
