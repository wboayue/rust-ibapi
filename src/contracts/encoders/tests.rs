#[test]
fn test_encode_request_market_rule() {
    let results = super::encode_request_market_rule(26);

    match results {
        Ok(message) => {
            assert_eq!(message.encode(), "91\026\0", "message.encode()");
        }
        Err(err) => {
            assert!(false, "error encoding market rule request: {err}");
        }
    }
}
