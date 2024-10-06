use crate::ToField;

use super::*;

#[test]
fn request_positions() {
    let results = super::request_positions();

    match results {
        Ok(message) => {
            assert_eq!(message[0], OutgoingMessages::RequestPositions.to_field(), "message.type");
            assert_eq!(message[1], "1", "message.version");
        }
        Err(err) => {
            assert!(false, "error encoding request positions: {err}");
        }
    }
}

#[test]
fn cancel_positions() {
    let results = super::cancel_positions();

    match results {
        Ok(message) => {
            assert_eq!(message[0], OutgoingMessages::CancelPositions.to_field(), "message.type");
            assert_eq!(message[1], "1", "message.version");
        }
        Err(err) => {
            assert!(false, "error encoding cancel positions: {err}");
        }
    }
}

#[test]
fn request_family_codes() {
    let results = super::request_family_codes();

    match results {
        Ok(message) => {
            assert_eq!(message[0], OutgoingMessages::RequestFamilyCodes.to_field(), "message.type");
            assert_eq!(message[1], "1", "message.version");
        }
        Err(err) => {
            assert!(false, "error encoding request family codes: {err}");
        }
    }
}

#[test]
fn test_encode_request_pnl() {
    let request_id = 3000;
    let account = "DU1234567";
    let model_code: Option<&str> = None;

    let request = super::encode_request_pnl(request_id, &account, model_code).expect("encode request pnl failed");

    assert_eq!(request[0], OutgoingMessages::RequestPnL.to_field(), "message.type");
    assert_eq!(request[1], request_id.to_field(), "message.request_id");
    assert_eq!(request[2], account, "message.account");
    assert_eq!(request[3], "", "message.model_code");
}

#[test]
fn test_encode_request_pnl_single() {
    let request_id = 3000;
    let account = "DU1234567";
    let model_code: Option<&str> = None;
    let contract_id = 1001;

    let request = super::encode_request_pnl_single(request_id, &account, contract_id, model_code).expect("encode request pnl failed");

    assert_eq!(request[0], OutgoingMessages::RequestPnLSingle.to_field(), "message.type");
    assert_eq!(request[1], request_id.to_field(), "message.request_id");
    assert_eq!(request[2], account, "message.account");
    assert_eq!(request[3], "", "message.model_code");
    assert_eq!(request[4], contract_id.to_field(), "message.contract_id");
}
