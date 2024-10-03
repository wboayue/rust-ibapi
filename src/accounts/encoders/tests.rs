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
