use super::*;

#[test]
fn test_encode_request_contract_data() {}

#[test]
fn test_decode_contract_details() {}

#[test]
fn test_read_last_trade_date() {
    let mut contract = ContractDetails::default();

    // handles blank string
    let result = read_last_trade_date(&mut contract, "", false);
    assert!(!result.is_err(), "unexpected error {:?}", result);

    // handles non bond contracts

    // handles bond contracts
}

#[test]
fn test_encode_request_matching_symbols() {}

#[test]
fn test_decode_contract_descriptions() {}
