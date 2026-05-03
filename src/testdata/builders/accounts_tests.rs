//! Builder tests focused on text wire-format invariants and conditional-emit logic.
//!
//! Standalone proto round-trip tests are intentionally absent: they amount to
//! `builder → prost → assert` and don't exercise production code. End-to-end
//! coverage of `builder → production decoder → domain object` lives in
//! `crate::accounts::common::decoders::tests`. End-to-end request coverage
//! (client API → captured bytes → `assert_request<B>`) lives in
//! `crate::accounts::{sync,async}::tests`.

use super::*;
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_TICKER_ID};
use crate::testdata::builders::ResponseEncoder;

fn split_pipe(s: &str) -> Vec<&str> {
    s.split_terminator('|').collect()
}

#[test]
fn managed_accounts_default_emits_csv() {
    let encoded = managed_accounts().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "15");
    assert_eq!(fields[1], "1");
    assert_eq!(fields[2], format!("{TEST_ACCOUNT},DU7654321"));
}

#[test]
fn managed_accounts_setter_overrides_csv() {
    let encoded = managed_accounts().accounts(["A", "B", "C"]).encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[2], "A,B,C");
}

#[test]
fn account_summary_default_field_order() {
    let encoded = account_summary().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "63");
    assert_eq!(fields[1], "1");
    assert_eq!(fields[2], TEST_TICKER_ID.to_string());
    assert_eq!(fields[3], TEST_ACCOUNT);
    assert_eq!(fields[4], "AccountType");
    assert_eq!(fields[5], "FA");
    assert_eq!(fields[6], "");
}

#[test]
fn account_summary_end_default_emits_request_id() {
    assert_eq!(account_summary_end().encode_pipe(), format!("64|1|{TEST_TICKER_ID}|"));
}

// AccountValue's wire format encodes a version field that depends on whether
// `account` is set. The conditional-emit logic below isn't a tautology — it
// matches the production decoder's version-gated parsing in
// `decode_account_value`.

#[test]
fn account_value_v1_default_omits_account_field() {
    let encoded = account_value().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "6");
    assert_eq!(fields[1], "1", "version 1 when no account is set");
    assert_eq!(fields[2], "CashBalance");
    assert_eq!(fields[3], "1000.00");
    assert_eq!(fields[4], "USD");
    assert_eq!(fields.len(), 5);
}

#[test]
fn account_value_v2_when_account_set() {
    let encoded = account_value().account(TEST_ACCOUNT).encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[1], "2", "version bumped when account is present");
    assert_eq!(fields[5], TEST_ACCOUNT);
    assert_eq!(fields.len(), 6);
}

#[test]
fn account_download_end_default() {
    assert_eq!(account_download_end().encode_pipe(), format!("54|1|{TEST_ACCOUNT}|"));
}

#[test]
fn account_update_multi_default_field_order() {
    let encoded = account_update_multi().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "73");
    assert_eq!(fields[1], "1");
    assert_eq!(fields[2], TEST_TICKER_ID.to_string());
    assert_eq!(fields[3], TEST_ACCOUNT);
    assert_eq!(fields[4], "");
    assert_eq!(fields[5], "CashBalance");
    assert_eq!(fields[6], "94629.71");
    assert_eq!(fields[7], "USD");
}

#[test]
fn account_update_multi_end_default() {
    assert_eq!(account_update_multi_end().encode_pipe(), format!("74|1|{TEST_TICKER_ID}|"));
}

#[test]
fn family_codes_empty_default() {
    assert_eq!(family_codes().encode_pipe(), "78|0|");
}

#[test]
fn family_codes_with_entries() {
    let encoded = family_codes().push("ACC1", "FC1").push("ACC2", "FC2").encode_pipe();
    assert_eq!(encoded, "78|2|ACC1|FC1|ACC2|FC2|");
}

#[test]
fn current_time_default_text() {
    assert_eq!(current_time().encode_pipe(), "49|1|1678890000|");
}

#[test]
fn pnl_default_emits_all_fields() {
    let encoded = pnl().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "94");
    assert_eq!(fields[1], TEST_TICKER_ID.to_string());
    assert_eq!(fields[2], "1234.56");
    assert_eq!(fields[3], "500");
    assert_eq!(fields[4], "250");
    assert_eq!(fields.len(), 5);
}

#[test]
fn pnl_omits_optional_pnl_when_none() {
    let encoded = pnl().unrealized_pnl(None).realized_pnl(None).encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields.len(), 3, "only msg/req_id/daily emitted");
}

#[test]
fn pnl_single_default_field_order() {
    let encoded = pnl_single().encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields[0], "95");
    assert_eq!(fields[1], TEST_TICKER_ID.to_string());
    assert_eq!(fields[2], "100");
    assert_eq!(fields[3], "50");
    assert_eq!(fields[4], "25");
    assert_eq!(fields[5], "10");
    assert_eq!(fields[6], "12345.67");
}
