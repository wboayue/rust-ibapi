//! Builder tests focused on text wire-format invariants.
//!
//! Proto round-trip tests are intentionally absent — they don't exercise any
//! production code. End-to-end coverage of `builder → production decoder →
//! domain object` lives in `crate::contracts::common::decoders::tests`.
//! End-to-end request coverage (client API → captured bytes →
//! `assert_request<B>`) lives in `crate::contracts::{sync,async}::tests`.

use super::*;
use crate::testdata::builders::ResponseEncoder;

fn split_pipe(s: &str) -> Vec<&str> {
    s.split_terminator('|').collect()
}

#[test]
fn contract_data_end_default_field_order() {
    // request_id_response_builder! defaults request_id to TEST_TICKER_ID (100);
    // override here to keep the assertion stable if that constant moves.
    let encoded = contract_data_end().request_id(9000).encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields, vec!["52", "1", "9000"]);
}

#[test]
fn option_chain_end_omits_version_field() {
    // OptionChainEnd has request_id at index 1, unlike most *End sentinels which
    // sit at index 2 after a "1" version. Lock the wire shape down.
    let encoded = option_chain_end().request_id(9123).encode_pipe();
    let fields = split_pipe(&encoded);
    assert_eq!(fields, vec!["76", "9123"]);
}
