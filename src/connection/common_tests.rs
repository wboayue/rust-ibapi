use super::*;
use std::sync::{Arc, Mutex};
use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, TimeZone};

const TEST_SERVER_VERSION: i32 = server_versions::PROTOBUF;

fn empty_callbacks<'a>() -> StartupCallbacks<'a> {
    StartupCallbacks { startup: None, notice: None }
}

fn startup_callbacks<'a>(cb: &'a (dyn Fn(StartupMessage) + Send + Sync)) -> StartupCallbacks<'a> {
    StartupCallbacks {
        startup: Some(cb),
        notice: None,
    }
}

fn notice_callbacks<'a>(cb: &'a (dyn Fn(Notice) + Send + Sync)) -> StartupCallbacks<'a> {
    StartupCallbacks {
        startup: None,
        notice: Some(cb),
    }
}

#[test]
fn test_parse_account_info_next_valid_id() {
    let handler = ConnectionHandler::default();
    // NextValidId message: message_type=9, version=1, next_order_id=1000
    let mut message = ResponseMessage::from("9\01\01000\0");

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_callbacks());
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.next_order_id, Some(1000));
    assert_eq!(info.managed_accounts, None);
}

#[test]
fn test_parse_account_info_managed_accounts() {
    let handler = ConnectionHandler::default();
    // ManagedAccounts message: message_type=15, version=1, accounts="DU123,DU456"
    let mut message = ResponseMessage::from("15\01\0DU123,DU456\0");

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_callbacks());
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.next_order_id, None);
    assert_eq!(info.managed_accounts, Some("DU123,DU456".to_string()));
}

#[test]
fn test_dispatch_unsolicited_open_order_invokes_callback() {
    // Sparse OpenOrder frame — decoder fails, so the callback receives
    // `StartupMessage::Other` with message_type OpenOrder. Either way, callback
    // fires. The exhaustive typed-decode test uses a realistic frame below.
    let mut message = ResponseMessage::from("5\0123\0AAPL\0STK\0");

    let captured: Arc<Mutex<Option<IncomingMessages>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        let mt = match msg {
            StartupMessage::OpenOrder(_) => IncomingMessages::OpenOrder,
            StartupMessage::OrderStatus(_) => IncomingMessages::OrderStatus,
            StartupMessage::OpenOrderEnd => IncomingMessages::OpenOrderEnd,
            StartupMessage::AccountUpdate(_) => IncomingMessages::AccountValue,
            StartupMessage::Other(rm) => rm.message_type(),
        };
        *captured_clone.lock().unwrap() = Some(mt);
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert_eq!(*captured.lock().unwrap(), Some(IncomingMessages::OpenOrder));
}

#[test]
fn test_dispatch_unsolicited_order_status_invokes_callback() {
    let mut message = ResponseMessage::from("3\0456\0Filled\0100\0");

    let captured: Arc<Mutex<Option<IncomingMessages>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        let mt = match msg {
            StartupMessage::OpenOrder(_) => IncomingMessages::OpenOrder,
            StartupMessage::OrderStatus(_) => IncomingMessages::OrderStatus,
            StartupMessage::OpenOrderEnd => IncomingMessages::OpenOrderEnd,
            StartupMessage::AccountUpdate(_) => IncomingMessages::AccountValue,
            StartupMessage::Other(rm) => rm.message_type(),
        };
        *captured.lock().unwrap() = Some(mt);
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    // Decoder may fail on truncated frame → Other(rm) with OrderStatus type.
    assert_eq!(*captured_clone.lock().unwrap(), Some(IncomingMessages::OrderStatus));
}

#[test]
fn test_dispatch_unsolicited_account_value_typed() {
    // AccountValue text frame: msg_type=6, version=2, key, value, currency, account
    let mut message = ResponseMessage::from("6\02\0NetLiquidation\0123456.78\0USD\0DU1234567\0");

    let captured: Arc<Mutex<Option<crate::accounts::AccountValue>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        if let StartupMessage::AccountUpdate(crate::accounts::AccountUpdate::AccountValue(av)) = msg {
            *captured.lock().unwrap() = Some(av);
        } else {
            panic!("expected AccountUpdate::AccountValue, got {msg:?}");
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    let got = captured_clone.lock().unwrap().take().expect("callback didn't fire");
    assert_eq!(got.key, "NetLiquidation");
    assert_eq!(got.value, "123456.78");
    assert_eq!(got.currency, "USD");
    assert_eq!(got.account.as_deref(), Some("DU1234567"));
}

#[test]
fn test_dispatch_unsolicited_open_order_end_typed() {
    // OpenOrderEnd: msg_type=53, version=1
    let mut message = ResponseMessage::from("53\01\0");

    let captured: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        if matches!(msg, StartupMessage::OpenOrderEnd) {
            *captured.lock().unwrap() = true;
        } else {
            panic!("expected OpenOrderEnd, got {msg:?}");
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert!(*captured_clone.lock().unwrap(), "OpenOrderEnd not delivered as typed variant");
}

#[test]
fn test_dispatch_unsolicited_account_download_end_typed() {
    // AccountDownloadEnd: msg_type=54, version=1, account
    let mut message = ResponseMessage::from("54\01\0DU1234567\0");

    let captured: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        if matches!(msg, StartupMessage::AccountUpdate(crate::accounts::AccountUpdate::End)) {
            *captured.lock().unwrap() = true;
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert!(*captured_clone.lock().unwrap(), "End variant not delivered");
}

#[test]
fn test_dispatch_unsolicited_unknown_falls_to_other() {
    // CompletedOrder (msg_type=101) — no typed variant in StartupMessage yet,
    // so it should fall through to Other.
    let mut message = ResponseMessage::from("101\0\0");

    let captured: Arc<Mutex<Option<IncomingMessages>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| {
        if let StartupMessage::Other(rm) = msg {
            *captured.lock().unwrap() = Some(rm.message_type());
        } else {
            panic!("expected Other for unknown message type, got {msg:?}");
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert_eq!(*captured_clone.lock().unwrap(), Some(IncomingMessages::CompletedOrder));
}

#[test]
fn test_dispatch_unsolicited_notice_warning_invokes_notice_callback() {
    // Error frame with code 2104 (farm-status warning).
    // Format: msg_type=4, version=2, request_id=-1, code=2104, message
    let mut message = ResponseMessage::from("4\02\0-1\02104\0Market data farm OK\0");

    let captured: Arc<Mutex<Option<Notice>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |notice: Notice| {
        *captured.lock().unwrap() = Some(notice);
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_callbacks(&cb));
    let got = captured_clone.lock().unwrap().take().expect("notice callback didn't fire");
    assert_eq!(got.code, 2104);
    assert_eq!(got.message, "Market data farm OK");
}

#[test]
fn test_dispatch_unsolicited_notice_hard_error_invokes_notice_callback() {
    // Error frame with code 504 — non-warning, non-system message.
    let mut message = ResponseMessage::from("4\02\0-1\0504\0Not connected\0");

    let captured: Arc<Mutex<Option<Notice>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |notice: Notice| {
        *captured.lock().unwrap() = Some(notice);
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_callbacks(&cb));
    let got = captured_clone.lock().unwrap().take().expect("notice callback didn't fire");
    assert_eq!(got.code, 504);
}

#[test]
fn test_dispatch_unsolicited_notice_only_fires_notice_callback() {
    // Error frame should fire the notice callback, NOT the startup callback.
    let mut message = ResponseMessage::from("4\02\0-1\02104\0farm OK\0");

    let startup_fired = Arc::new(Mutex::new(false));
    let startup_fired_clone = startup_fired.clone();
    let startup_cb = move |_msg: StartupMessage| {
        *startup_fired_clone.lock().unwrap() = true;
    };
    let notice_fired = Arc::new(Mutex::new(false));
    let notice_fired_clone = notice_fired.clone();
    let notice_cb = move |_notice: Notice| {
        *notice_fired_clone.lock().unwrap() = true;
    };

    dispatch_unsolicited_message(
        TEST_SERVER_VERSION,
        &mut message,
        &StartupCallbacks {
            startup: Some(&startup_cb),
            notice: Some(&notice_cb),
        },
    );
    assert!(!*startup_fired.lock().unwrap(), "startup callback should not fire on Error");
    assert!(*notice_fired.lock().unwrap(), "notice callback should fire on Error");
}

#[test]
fn test_parse_account_info_callback_not_invoked_for_next_valid_id() {
    // NextValidId is consumed internally — neither callback fires.
    let handler = ConnectionHandler::default();
    let mut message = ResponseMessage::from("9\01\01000\0");

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();
    let cb = move |_: StartupMessage| {
        *fired_clone.lock().unwrap() = true;
    };

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert!(result.is_ok());
    assert!(!*fired.lock().unwrap(), "callback should NOT be invoked for NextValidId");
}

#[test]
fn test_parse_account_info_callback_not_invoked_for_managed_accounts() {
    let handler = ConnectionHandler::default();
    let mut message = ResponseMessage::from("15\01\0DU123\0");

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();
    let cb = move |_: StartupMessage| {
        *fired_clone.lock().unwrap() = true;
    };

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &startup_callbacks(&cb));
    assert!(result.is_ok());
    assert!(!*fired.lock().unwrap(), "callback should NOT be invoked for ManagedAccounts");
}

#[test]
fn test_parse_account_info_multiple_messages_callback() {
    let handler = ConnectionHandler::default();
    let count = Arc::new(Mutex::new(0));
    let count_clone = count.clone();
    let cb = move |_: StartupMessage| {
        *count_clone.lock().unwrap() += 1;
    };
    let cbs = startup_callbacks(&cb);

    // First message: OpenOrder (sparse → Other)
    let mut msg1 = ResponseMessage::from("5\0123\0AAPL\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg1, &cbs).unwrap();

    // Second message: OrderStatus (sparse → Other)
    let mut msg2 = ResponseMessage::from("3\0456\0Filled\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg2, &cbs).unwrap();

    // Third message: NextValidId (should NOT trigger callback)
    let mut msg3 = ResponseMessage::from("9\01\01000\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg3, &cbs).unwrap();

    assert_eq!(*count.lock().unwrap(), 2, "callback should be invoked exactly twice");
}

#[test]
fn test_require_protobuf_support_accepts_minimum() {
    require_protobuf_support(server_versions::PROTOBUF).expect("PROTOBUF version must be accepted");
}

#[test]
fn test_require_protobuf_support_accepts_newer() {
    require_protobuf_support(server_versions::PROTOBUF + 5).expect("newer versions must be accepted");
}

#[test]
fn test_require_protobuf_support_rejects_older() {
    let actual = server_versions::PROTOBUF - 1;
    let err = require_protobuf_support(actual).expect_err("older versions must be rejected");

    match &err {
        Error::ServerVersion(required, got, msg) => {
            assert_eq!(*required, server_versions::PROTOBUF);
            assert_eq!(*got, actual);
            assert!(msg.contains("protobuf"), "message should mention protobuf: {msg}");
            assert!(msg.contains("upgrade"), "message should tell user to upgrade: {msg}");
        }
        other => panic!("expected Error::ServerVersion, got {other:?}"),
    }

    let rendered = err.to_string();
    let expected_required = format!("server version {} required", server_versions::PROTOBUF);
    assert!(rendered.contains(&expected_required), "rendered: {rendered}");
    assert!(rendered.contains(&actual.to_string()), "rendered: {rendered}");
}

#[test]
fn test_parse_connection_time() {
    let example = "20230405 22:20:39 PST";
    let (connection_time, _) = parse_connection_time(example).unwrap();

    let la = timezones::db::america::LOS_ANGELES;
    if let OffsetResult::Some(other) = datetime!(2023-04-05 22:20:39).assume_timezone(la) {
        assert_eq!(connection_time, Some(other));
    }
}

#[test]
fn test_parse_connection_time_china_standard_time() {
    let example = "20230405 22:20:39 China Standard Time";
    let (connection_time, timezone) = parse_connection_time(example).unwrap();

    assert!(connection_time.is_some());
    assert!(timezone.is_some());
    assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
}

#[test]
fn test_parse_connection_time_chinese_utf8() {
    let example = "20230405 22:20:39 中国标准时间";
    let (connection_time, timezone) = parse_connection_time(example).unwrap();

    assert!(connection_time.is_some());
    assert!(timezone.is_some());
    assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
}

#[test]
fn test_parse_connection_time_mojibake() {
    // Simulate GB2312 timezone decoded as UTF-8 lossy
    let example = "20230405 22:20:39 \u{FFFD}\u{FFFD}\u{FFFD}";
    let (connection_time, timezone) = parse_connection_time(example).unwrap();

    assert!(connection_time.is_some());
    assert!(timezone.is_some());
    assert_eq!(timezone.unwrap().name(), "Asia/Shanghai");
}

#[test]
fn test_parse_connection_time_unknown_timezone_errors() {
    let example = "20230405 22:20:39 Bogus Standard Time";
    let err = parse_connection_time(example).expect_err("unknown tz must error");

    assert!(matches!(err, Error::UnsupportedTimeZone(ref name) if name == "Bogus Standard Time"));
    let rendered = err.to_string();
    assert!(rendered.contains("Bogus Standard Time"), "missing tz name: {rendered}");
    assert!(
        rendered.contains("register_timezone_alias"),
        "missing programmatic-fix pointer: {rendered}"
    );
    assert!(rendered.contains("IBAPI_TIMEZONE_ALIASES"), "missing env-var pointer: {rendered}");
    assert!(
        rendered.contains("github.com/wboayue/rust-ibapi"),
        "missing issue-tracker pointer: {rendered}"
    );
}

#[test]
fn test_parse_connection_time_short_input_still_ok() {
    // Truncated wire data — preserve current tolerance, no error.
    let (time, tz) = parse_connection_time("20230405").unwrap();
    assert!(time.is_none());
    assert!(tz.is_none());
}

#[test]
fn test_parse_connection_time_unparseable_date_still_ok() {
    // Timezone resolves; only the wall-clock fails. Preserve tolerance.
    let (time, tz) = parse_connection_time("BADDATE 99:99:99 PST").unwrap();
    assert!(time.is_none());
    assert!(tz.is_some());
}

#[test]
fn test_connection_handler_handshake() {
    let handler = ConnectionHandler::default();
    let handshake = handler.format_handshake();

    // Should start with "API\0"
    assert_eq!(&handshake[0..4], b"API\0");

    // Should contain version string
    let version_part = &handshake[4..];
    assert!(!version_part.is_empty());
}

#[test]
fn test_connection_handler_start_api() {
    use crate::messages::PROTOBUF_MSG_ID;

    let handler = ConnectionHandler::default();
    let data = handler.format_start_api(123, server_versions::PROTOBUF);

    // First 4 bytes: big-endian (StartApi=71 + 200)
    let msg_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(msg_id, 71 + PROTOBUF_MSG_ID);

    // Remaining bytes: protobuf-encoded StartApiRequest with client_id=123
    let request: crate::proto::StartApiRequest = prost::Message::decode(&data[4..]).unwrap();
    assert_eq!(request.client_id, Some(123));
}

#[test]
fn test_connection_handler_start_api_protobuf() {
    let handler = ConnectionHandler::default();
    let data = handler.format_start_api(123, server_versions::PROTOBUF);

    // First 4 bytes: msg_id (71 + 200 = 271)
    let msg_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(msg_id, 271);

    // Decode protobuf payload
    use prost::Message;
    let req = crate::proto::StartApiRequest::decode(&data[4..]).unwrap();
    assert_eq!(req.client_id, Some(123));
}

#[test]
fn test_parse_raw_message_protobuf() {
    use crate::messages::PROTOBUF_MSG_ID;

    // Simulate a protobuf message: msg_id = 5 + 200 = 205, then some payload
    let msg_id: i32 = 5 + PROTOBUF_MSG_ID;
    let payload = vec![0x08, 0x64]; // varint tag=1, value=100
    let mut data = msg_id.to_be_bytes().to_vec();
    data.extend_from_slice(&payload);

    let (message, trace_str) = parse_raw_message(&data, server_versions::PROTOBUF);
    assert!(message.is_protobuf);
    assert_eq!(message.message_type(), IncomingMessages::OpenOrder);
    assert_eq!(message.raw_bytes(), Some(payload.as_slice()));
    assert!(trace_str.is_none()); // no trace string for protobuf
}

#[test]
fn test_parse_raw_message_binary_id_text_payload() {
    // Simulate a text message at server >= 201: binary msg_id=9, then NUL-delimited text
    let msg_id: i32 = 9; // NextValidId
    let text_payload = b"1\01000\0";
    let mut data = msg_id.to_be_bytes().to_vec();
    data.extend_from_slice(text_payload);

    let (message, trace_str) = parse_raw_message(&data, server_versions::PROTOBUF);
    assert!(!message.is_protobuf);
    assert_eq!(message.message_type(), IncomingMessages::NextValidId);
    assert_eq!(message.peek_string(1), "1"); // version field
    assert_eq!(message.peek_int(2).unwrap(), 1000); // next_order_id
    assert!(trace_str.is_some());
}

#[test]
fn test_parse_raw_message_legacy_text() {
    let data = b"9\01\01000\0";
    let (message, trace_str) = parse_raw_message(data, 173); // server < PROTOBUF

    assert!(!message.is_protobuf);
    assert_eq!(message.message_type(), IncomingMessages::NextValidId);
    assert!(trace_str.is_some());
}

/// Test handling of non-UTF8 encoded data from IB Gateway (issue #352)
/// Some IB Gateway installations send timezone names in GB2312/GBK encoding
/// (e.g., Chinese "中国标准时间" for "China Standard Time")
#[test]
fn test_non_utf8_handshake_response() {
    // Actual bytes from issue #352: "173\020251205 23:13:45 中国标准时间\0"
    // where the Chinese characters are GB2312 encoded, not UTF-8
    let gb2312_bytes: Vec<u8> = vec![
        49, 55, 51, 0, // "173\0" - server version
        50, 48, 50, 53, 49, 50, 48, 53, 32, // "20251205 " - date
        50, 51, 58, 49, 51, 58, 52, 53, 32, // "23:13:45 " - time
        214, 208, 185, 250, 177, 234, 215, 188, 202, 177, 188, 228, // GB2312: 中国标准时间
        0,   // null terminator
    ];

    // from_utf8_lossy should handle this without error
    let raw_string = String::from_utf8_lossy(&gb2312_bytes).into_owned();

    // Should contain the ASCII portions intact
    assert!(raw_string.contains("173"));
    assert!(raw_string.contains("20251205"));
    assert!(raw_string.contains("23:13:45"));

    // Non-UTF8 bytes are replaced with replacement character
    assert!(raw_string.contains('\u{FFFD}'));

    // Parse as ResponseMessage and extract handshake data
    let mut response = ResponseMessage::from(&raw_string);
    let handler = ConnectionHandler::default();
    let result = handler.parse_handshake_response(&mut response);

    assert!(result.is_ok());
    let handshake_data = result.unwrap();
    assert_eq!(handshake_data.server_version, 173);
    // server_time will contain replacement characters but parsing succeeds
    assert!(handshake_data.server_time.contains("20251205"));
}

#[test]
fn test_connection_options_default() {
    let opts = ConnectionOptions::default();
    assert_eq!(opts.tcp_no_delay, false);
    assert!(opts.startup_callback.is_none());
    assert!(opts.startup_notice_callback.is_none());
}

#[test]
fn test_connection_options_builder() {
    let opts = ConnectionOptions::default()
        .tcp_no_delay(true)
        .startup_callback(|_msg: StartupMessage| {})
        .startup_notice_callback(|_notice: Notice| {});
    assert_eq!(opts.tcp_no_delay, true);
    assert!(opts.startup_callback.is_some());
    assert!(opts.startup_notice_callback.is_some());
}

#[test]
fn test_connection_options_clone() {
    let opts = ConnectionOptions::default().tcp_no_delay(true);
    let cloned = opts.clone();
    assert_eq!(cloned.tcp_no_delay, true);
}

#[test]
fn test_connection_options_debug() {
    let opts = ConnectionOptions::default().tcp_no_delay(true);
    let debug_str = format!("{:?}", opts);
    assert!(debug_str.contains("tcp_no_delay: true"));
    assert!(debug_str.contains("startup_callback: false"));
    assert!(debug_str.contains("startup_notice_callback: false"));
}
