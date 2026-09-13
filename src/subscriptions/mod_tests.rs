use super::*;

#[test]
fn test_log_cancel_error_classifies_session_down() {
    let cases = [
        (Error::ConnectionReset, true),
        (Error::Shutdown, true),
        (Error::Cancelled, false),
        (Error::Simple("boom".to_string()), false),
    ];

    for (error, session_down) in cases {
        assert_eq!(is_session_down_error(&error), session_down, "{error:?}");
        log_cancel_error("subscription", &error);
    }
}
