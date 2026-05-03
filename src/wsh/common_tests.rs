use super::common::decoders;
use super::*;
use crate::messages::ResponseMessage;

#[test]
fn test_autofill_is_specified() {
    assert!(!AutoFill::default().is_specified());

    assert!(AutoFill {
        competitors: true,
        portfolio: false,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: true,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: false,
        watchlist: true,
    }
    .is_specified());
}

#[test]
fn test_autofill_combinations() {
    let combinations = vec![
        (false, false, false, false),
        (true, false, false, true),
        (false, true, false, true),
        (false, false, true, true),
        (true, true, false, true),
        (true, false, true, true),
        (false, true, true, true),
        (true, true, true, true),
    ];

    for (competitors, portfolio, watchlist, expected) in combinations {
        let autofill = AutoFill {
            competitors,
            portfolio,
            watchlist,
        };
        assert_eq!(
            autofill.is_specified(),
            expected,
            "Failed for combination: competitors={competitors}, portfolio={portfolio}, watchlist={watchlist}",
        );
    }
}

#[test]
fn test_decode_wsh_metadata() {
    let message = ResponseMessage::from("104\09000\0{\"test\":\"data\"}\0");
    let result = decoders::decode_wsh_metadata(message).unwrap();
    assert_eq!(result.data_json, "{\"test\":\"data\"}");
}

#[test]
fn test_decode_wsh_event_data() {
    let message = ResponseMessage::from("105\09000\0{\"test\":\"data\"}\0");
    let result = decoders::decode_wsh_event_data(message).unwrap();
    assert_eq!(result.data_json, "{\"test\":\"data\"}");
}

#[test]
fn test_decode_wsh_metadata_empty_json() {
    let message = ResponseMessage::from("104\09000\0\0");
    let result = decoders::decode_wsh_metadata(message).unwrap();
    assert_eq!(result.data_json, "");
}

#[test]
fn test_decode_wsh_event_data_empty_json() {
    let message = ResponseMessage::from("105\09000\0\0");
    let result = decoders::decode_wsh_event_data(message).unwrap();
    assert_eq!(result.data_json, "");
}

#[test]
fn test_decode_wsh_metadata_with_special_chars() {
    let message = ResponseMessage::from("104\09000\0{\"data\":\"test\\nwith\\tspecial\\rchars\"}\0");
    let result = decoders::decode_wsh_metadata(message).unwrap();
    assert_eq!(result.data_json, "{\"data\":\"test\\nwith\\tspecial\\rchars\"}");
}
