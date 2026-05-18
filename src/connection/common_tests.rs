use super::*;
use crate::messages::{HANDSHAKE_DECODE_FAILURE_CODE, HANDSHAKE_UNKNOWN_FRAME_CODE};
use std::sync::{Arc, Mutex};
use time::macros::datetime;
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt, TimeZone};

const TEST_SERVER_VERSION: i32 = server_versions::PROTOBUF_SCAN_DATA;

/// Test sink that drops every notice. Used when the test cares about the
/// startup callback, not the notice fan-out.
#[derive(Default)]
struct DiscardingSink;
impl NoticeSink for DiscardingSink {
    fn deliver(&self, _: Notice) {}
}

/// Test sink that captures every notice into a shared `Vec`.
#[derive(Default)]
struct CapturingSink {
    notices: Mutex<Vec<Notice>>,
}
impl CapturingSink {
    fn last(&self) -> Option<Notice> {
        self.notices.lock().unwrap().last().cloned()
    }
    fn count(&self) -> usize {
        self.notices.lock().unwrap().len()
    }
}
impl NoticeSink for CapturingSink {
    fn deliver(&self, n: Notice) {
        self.notices.lock().unwrap().push(n);
    }
}

fn empty_ctx<'a>() -> StartupHandshakeContext<'a> {
    static SINK: DiscardingSink = DiscardingSink;
    StartupHandshakeContext {
        startup: None,
        notice_sink: &SINK,
    }
}

fn startup_ctx<'a>(cb: &'a (dyn Fn(StartupMessage) + Send + Sync)) -> StartupHandshakeContext<'a> {
    static SINK: DiscardingSink = DiscardingSink;
    StartupHandshakeContext {
        startup: Some(cb),
        notice_sink: &SINK,
    }
}

fn notice_sink_ctx(sink: &CapturingSink) -> StartupHandshakeContext<'_> {
    StartupHandshakeContext {
        startup: None,
        notice_sink: sink,
    }
}

/// Context with both a startup callback AND a capturing notice sink. Used to
/// verify decode-failure routing (callback should NOT fire; sink should
/// receive the synthesized notice).
fn full_ctx<'a>(cb: &'a (dyn Fn(StartupMessage) + Send + Sync), sink: &'a CapturingSink) -> StartupHandshakeContext<'a> {
    StartupHandshakeContext {
        startup: Some(cb),
        notice_sink: sink,
    }
}

#[test]
fn test_parse_account_info_next_valid_id() {
    let handler = ConnectionHandler::default();
    // NextValidId message: message_type=9, version=1, next_order_id=1000
    let mut message = ResponseMessage::from("9\01\01000\0");

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx());
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

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx());
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.next_order_id, None);
    assert_eq!(info.managed_accounts, Some("DU123,DU456".to_string()));
}

#[test]
fn test_dispatch_unsolicited_open_order_decode_failure_emits_notice() {
    // Sparse text-framed OpenOrder — decoder calls require_proto() and rejects.
    let mut message = ResponseMessage::from("5\0123\0AAPL\0STK\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(
        notice.message.contains("OpenOrder"),
        "notice message should name the kind: {}",
        notice.message
    );
}

#[test]
fn test_dispatch_unsolicited_order_status_decode_failure_emits_notice() {
    let mut message = ResponseMessage::from("3\0456\0Filled\0100\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(notice.message.contains("OrderStatus"), "notice should name the kind: {}", notice.message);
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

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
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

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
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

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    assert!(*captured_clone.lock().unwrap(), "End variant not delivered");
}

#[test]
fn test_dispatch_unsolicited_unknown_emits_notice() {
    // NewsBulletins (msg_type=14) — no typed variant in StartupMessage.
    // Catch-all fires the notice sink with HANDSHAKE_UNKNOWN_FRAME_CODE.
    let mut message = ResponseMessage::from("14\0\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire for unknown handshake frame");
    let notice = sink.last().expect("notice sink should receive unknown-frame notice");
    assert_eq!(notice.code, HANDSHAKE_UNKNOWN_FRAME_CODE);
    assert!(
        notice.message.contains("NewsBulletins"),
        "notice should name the kind: {}",
        notice.message
    );
    assert!(notice.is_handshake_synthetic());
}

#[test]
fn test_dispatch_unsolicited_notice_warning_invokes_notice_sink() {
    // Error frame with code 2104 (farm-status warning).
    // Format: msg_type=4, version=2, request_id=-1, code=2104, message
    let mut message = ResponseMessage::from("4\02\0-1\02104\0Market data farm OK\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));

    let got = sink.last().expect("notice sink didn't receive notice");
    assert_eq!(got.code, 2104);
    assert_eq!(got.message, "Market data farm OK");
}

#[test]
fn test_dispatch_unsolicited_notice_hard_error_invokes_notice_sink() {
    // Error frame with code 504 — non-warning, non-system message.
    let mut message = ResponseMessage::from("4\02\0-1\0504\0Not connected\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));

    assert_eq!(sink.last().expect("sink missed").code, 504);
}

#[test]
fn test_dispatch_unsolicited_notice_only_fires_notice_sink() {
    // Error frame should fire the notice sink, NOT the startup callback.
    let mut message = ResponseMessage::from("4\02\0-1\02104\0farm OK\0");

    let startup_fired = Arc::new(Mutex::new(false));
    let startup_fired_clone = startup_fired.clone();
    let startup_cb = move |_msg: StartupMessage| {
        *startup_fired_clone.lock().unwrap() = true;
    };
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(
        TEST_SERVER_VERSION,
        &mut message,
        &StartupHandshakeContext {
            startup: Some(&startup_cb),
            notice_sink: &sink,
        },
    );
    assert!(!*startup_fired.lock().unwrap(), "startup callback should not fire on Error");
    assert_eq!(sink.count(), 1, "notice sink should receive exactly one notice on Error");
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

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
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

    let result = handler.parse_account_info(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
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
    let cbs = startup_ctx(&cb);

    // First message: OpenOrderEnd (unit marker — no decoder, callback fires).
    let mut msg1 = ResponseMessage::from("53\01\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg1, &cbs).unwrap();

    // Second message: CompletedOrdersEnd (unit marker — callback fires).
    let mut msg2 = ResponseMessage::from("102\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg2, &cbs).unwrap();

    // Third message: NextValidId (consumed internally — should NOT trigger callback)
    let mut msg3 = ResponseMessage::from("9\01\01000\0");
    handler.parse_account_info(TEST_SERVER_VERSION, &mut msg3, &cbs).unwrap();

    assert_eq!(*count.lock().unwrap(), 2, "callback should be invoked exactly twice");
}

#[test]
fn test_require_protobuf_support_accepts_minimum() {
    require_protobuf_support(server_versions::PROTOBUF_SCAN_DATA).expect("floor version must be accepted");
}

#[test]
fn test_require_protobuf_support_accepts_newer() {
    require_protobuf_support(server_versions::PROTOBUF_SCAN_DATA + 5).expect("newer versions must be accepted");
}

#[test]
fn test_require_protobuf_support_rejects_older() {
    let actual = server_versions::PROTOBUF_SCAN_DATA - 1;
    let err = require_protobuf_support(actual).expect_err("older versions must be rejected");

    match &err {
        Error::ServerVersion(required, got, msg) => {
            assert_eq!(*required, server_versions::PROTOBUF_SCAN_DATA);
            assert_eq!(*got, actual);
            assert!(msg.contains("protobuf"), "message should mention protobuf: {msg}");
            assert!(msg.contains("upgrade"), "message should tell user to upgrade: {msg}");
        }
        other => panic!("expected Error::ServerVersion, got {other:?}"),
    }

    let rendered = err.to_string();
    let expected_required = format!("server version {} required", server_versions::PROTOBUF_SCAN_DATA);
    assert!(rendered.contains(&expected_required), "rendered: {rendered}");
    assert!(rendered.contains(&actual.to_string()), "rendered: {rendered}");
}

#[test]
fn test_require_protobuf_support_rejects_previous_place_order_floor() {
    // Servers in [203, 209] don't yet emit the
    // CompletedOrder/ContractData/MarketData/AccountsPositions/HistoricalData/News
    // families in protobuf and we have no text-decode fallback, so the floor
    // must reject them.
    let err = require_protobuf_support(server_versions::PROTOBUF_PLACE_ORDER).expect_err("203 is below new floor");
    match err {
        Error::ServerVersion(required, got, _) => {
            assert_eq!(required, server_versions::PROTOBUF_SCAN_DATA);
            assert_eq!(got, server_versions::PROTOBUF_PLACE_ORDER);
        }
        other => panic!("expected Error::ServerVersion, got {other:?}"),
    }
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
    assert_eq!(message.peek_string(1).unwrap(), "1"); // version field
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
fn test_startup_message_message_type_typed_variants() {
    use crate::accounts::{AccountPortfolioValue, AccountUpdateTime, AccountValue};
    use crate::orders::{CommissionReport, ExecutionData, OrderData, OrderStatus};

    assert_eq!(
        StartupMessage::OpenOrder(OrderData::default()).message_type(),
        IncomingMessages::OpenOrder
    );
    assert_eq!(
        StartupMessage::OrderStatus(OrderStatus::default()).message_type(),
        IncomingMessages::OrderStatus
    );
    assert_eq!(StartupMessage::OpenOrderEnd.message_type(), IncomingMessages::OpenOrderEnd);
    assert_eq!(
        StartupMessage::AccountUpdate(AccountUpdate::AccountValue(AccountValue::default())).message_type(),
        IncomingMessages::AccountValue
    );
    assert_eq!(
        StartupMessage::AccountUpdate(AccountUpdate::PortfolioValue(AccountPortfolioValue::default())).message_type(),
        IncomingMessages::PortfolioValue
    );
    assert_eq!(
        StartupMessage::AccountUpdate(AccountUpdate::UpdateTime(AccountUpdateTime::default())).message_type(),
        IncomingMessages::AccountUpdateTime
    );
    assert_eq!(
        StartupMessage::AccountUpdate(AccountUpdate::End).message_type(),
        IncomingMessages::AccountDownloadEnd
    );
    assert_eq!(
        StartupMessage::Execution(ExecutionData::default()).message_type(),
        IncomingMessages::ExecutionData
    );
    assert_eq!(
        StartupMessage::CommissionReport(CommissionReport::default()).message_type(),
        IncomingMessages::CommissionsReport
    );
    assert_eq!(
        StartupMessage::CompletedOrder(OrderData::default()).message_type(),
        IncomingMessages::CompletedOrder
    );
    assert_eq!(StartupMessage::ExecutionDataEnd.message_type(), IncomingMessages::ExecutionDataEnd);
    assert_eq!(StartupMessage::CompletedOrdersEnd.message_type(), IncomingMessages::CompletedOrdersEnd);
}

#[test]
fn test_parse_account_info_next_valid_id_protobuf() {
    use prost::Message;

    let handler = ConnectionHandler::default();
    let proto = crate::proto::NextValidId { order_id: Some(4242) };
    let bytes = proto.encode_to_vec();
    let mut message = ResponseMessage::from_protobuf(IncomingMessages::NextValidId as i32, bytes, TEST_SERVER_VERSION);

    let info = handler
        .parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx())
        .expect("protobuf NextValidId must parse");
    assert_eq!(info.next_order_id, Some(4242));
    assert_eq!(info.managed_accounts, None);
}

#[test]
fn test_parse_account_info_managed_accounts_protobuf() {
    use prost::Message;

    let handler = ConnectionHandler::default();
    let proto = crate::proto::ManagedAccounts {
        accounts_list: Some("DU111,DU222".to_string()),
    };
    let bytes = proto.encode_to_vec();
    let mut message = ResponseMessage::from_protobuf(IncomingMessages::ManagedAccounts as i32, bytes, TEST_SERVER_VERSION);

    let info = handler
        .parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx())
        .expect("protobuf ManagedAccounts must parse");
    assert_eq!(info.next_order_id, None);
    assert_eq!(info.managed_accounts, Some("DU111,DU222".to_string()));
}

#[test]
fn test_parse_account_info_next_valid_id_protobuf_decode_error() {
    // Garbage bytes for the NextValidId proto envelope.
    let handler = ConnectionHandler::default();
    let mut message = ResponseMessage::from_protobuf(IncomingMessages::NextValidId as i32, vec![0xff, 0xff, 0xff], TEST_SERVER_VERSION);

    let err = handler
        .parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx())
        .expect_err("garbage protobuf must error");
    assert!(matches!(err, Error::ProtobufDecode(_)), "got {err:?}");
}

#[test]
fn test_parse_account_info_managed_accounts_protobuf_decode_error() {
    let handler = ConnectionHandler::default();
    let mut message = ResponseMessage::from_protobuf(IncomingMessages::ManagedAccounts as i32, vec![0xff, 0xff, 0xff], TEST_SERVER_VERSION);

    let err = handler
        .parse_account_info(TEST_SERVER_VERSION, &mut message, &empty_ctx())
        .expect_err("garbage protobuf must error");
    assert!(matches!(err, Error::ProtobufDecode(_)), "got {err:?}");
}

#[test]
fn test_dispatch_unsolicited_open_order_no_callback_is_noop() {
    let mut message = ResponseMessage::from("5\0123\0AAPL\0STK\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0, "OpenOrder must not deliver to notice sink");
}

#[test]
fn test_dispatch_unsolicited_order_status_no_callback_is_noop() {
    let mut message = ResponseMessage::from("3\0456\0Filled\0100\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_open_order_end_no_callback_is_noop() {
    let mut message = ResponseMessage::from("53\01\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_account_update_no_callback_is_noop() {
    let mut message = ResponseMessage::from("6\02\0NetLiquidation\0123.45\0USD\0DU1\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_account_value_decode_failure_emits_notice() {
    // Truncated AccountValue — too few fields to decode into AccountValue.
    let mut message = ResponseMessage::from("6\02\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(notice.message.contains("AccountValue"));
}

// =============================================================================
// New typed-variant dispatch tests (PR 2 of retire-response-message-public-surface)
// =============================================================================

/// Build a proto-frame ResponseMessage for the given testdata builder.
fn proto_frame<B: crate::testdata::builders::ResponseProtoEncoder>(kind: IncomingMessages, builder: &B) -> ResponseMessage {
    ResponseMessage::from_protobuf(kind as i32, builder.encode_proto(), TEST_SERVER_VERSION)
}

#[test]
fn test_dispatch_unsolicited_execution_typed() {
    use crate::testdata::builders::orders::ExecutionDataResponse;

    let builder = ExecutionDataResponse::default().symbol("TSLA").shares(100.0).price(196.52);
    let mut message = proto_frame(IncomingMessages::ExecutionData, &builder);

    let captured: Arc<Mutex<Option<crate::orders::ExecutionData>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| match msg {
        StartupMessage::Execution(e) => *captured.lock().unwrap() = Some(e),
        other => panic!("expected Execution, got {other:?}"),
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    let got = captured_clone.lock().unwrap().take().expect("callback didn't fire");
    assert_eq!(got.contract.symbol.to_string(), "TSLA");
    assert_eq!(got.execution.shares, 100.0);
    assert_eq!(got.execution.price, 196.52);
}

#[test]
fn test_dispatch_unsolicited_commission_report_typed() {
    use crate::testdata::builders::orders::CommissionReportResponse;

    let builder = CommissionReportResponse::default().commission(2.5).currency("USD");
    let mut message = proto_frame(IncomingMessages::CommissionsReport, &builder);

    let captured: Arc<Mutex<Option<crate::orders::CommissionReport>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| match msg {
        StartupMessage::CommissionReport(c) => *captured.lock().unwrap() = Some(c),
        other => panic!("expected CommissionReport, got {other:?}"),
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    let got = captured_clone.lock().unwrap().take().expect("callback didn't fire");
    assert_eq!(got.commission, 2.5);
    assert_eq!(got.currency, "USD");
}

#[test]
fn test_dispatch_unsolicited_completed_order_typed() {
    use crate::testdata::builders::orders::CompletedOrderResponse;

    let builder = CompletedOrderResponse::default();
    let mut message = proto_frame(IncomingMessages::CompletedOrder, &builder);

    let captured: Arc<Mutex<Option<crate::orders::OrderData>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    let cb = move |msg: StartupMessage| match msg {
        StartupMessage::CompletedOrder(o) => *captured.lock().unwrap() = Some(o),
        other => panic!("expected CompletedOrder, got {other:?}"),
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    let got = captured_clone.lock().unwrap().take().expect("callback didn't fire");
    // Completed orders carry the sentinel order_id = -1 per decode_completed_order_proto.
    assert_eq!(got.order_id, -1);
}

#[test]
fn test_dispatch_unsolicited_execution_data_end_typed() {
    // ExecutionDataEnd: msg_type=55, version=1, request_id=42
    let mut message = ResponseMessage::from("55\01\042\0");

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();
    let cb = move |msg: StartupMessage| {
        if matches!(msg, StartupMessage::ExecutionDataEnd) {
            *fired.lock().unwrap() = true;
        } else {
            panic!("expected ExecutionDataEnd, got {msg:?}");
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    assert!(*fired_clone.lock().unwrap(), "ExecutionDataEnd not delivered as typed variant");
}

#[test]
fn test_dispatch_unsolicited_completed_orders_end_typed() {
    // CompletedOrdersEnd: msg_type=102, no payload
    let mut message = ResponseMessage::from("102\0");

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();
    let cb = move |msg: StartupMessage| {
        if matches!(msg, StartupMessage::CompletedOrdersEnd) {
            *fired.lock().unwrap() = true;
        } else {
            panic!("expected CompletedOrdersEnd, got {msg:?}");
        }
    };

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &startup_ctx(&cb));
    assert!(*fired_clone.lock().unwrap(), "CompletedOrdersEnd not delivered as typed variant");
}

#[test]
fn test_dispatch_unsolicited_execution_decode_failure_emits_notice() {
    // Text-framed ExecutionData — decoder calls require_proto() and rejects it.
    let mut message = ResponseMessage::from("11\042\013\0AAPL\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(notice.message.contains("ExecutionData"));
}

#[test]
fn test_dispatch_unsolicited_commission_report_decode_failure_emits_notice() {
    let mut message = ResponseMessage::from("59\01\0EXEC0001.01.01\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(notice.message.contains("CommissionsReport"));
}

#[test]
fn test_dispatch_unsolicited_completed_order_decode_failure_emits_notice() {
    let mut message = ResponseMessage::from("101\0\0");

    let cb_fired = Arc::new(Mutex::new(false));
    let cb_fired_clone = cb_fired.clone();
    let cb = move |_msg: StartupMessage| *cb_fired_clone.lock().unwrap() = true;
    let sink = CapturingSink::default();

    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &full_ctx(&cb, &sink));

    assert!(!*cb_fired.lock().unwrap(), "callback must not fire on decode failure");
    let notice = sink.last().expect("notice sink should receive decode-failure notice");
    assert_eq!(notice.code, HANDSHAKE_DECODE_FAILURE_CODE);
    assert!(notice.message.contains("CompletedOrder"));
}

#[test]
fn test_dispatch_unsolicited_execution_no_callback_is_noop() {
    let mut message = ResponseMessage::from("11\042\013\0AAPL\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0, "ExecutionData must not deliver to notice sink");
}

#[test]
fn test_dispatch_unsolicited_commission_report_no_callback_is_noop() {
    let mut message = ResponseMessage::from("59\01\0EXEC0001.01.01\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_completed_order_no_callback_is_noop() {
    let mut message = ResponseMessage::from("101\0\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_execution_data_end_no_callback_is_noop() {
    let mut message = ResponseMessage::from("55\01\042\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_completed_orders_end_no_callback_is_noop() {
    let mut message = ResponseMessage::from("102\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    assert_eq!(sink.count(), 0);
}

#[test]
fn test_dispatch_unsolicited_unknown_no_callback_still_notices() {
    // NewsBulletins (14). Unknown handshake kind always fires the notice
    // sink regardless of callback presence — synthesized notices target
    // `Client::notice_stream()`, decoupled from the typed startup callback.
    let mut message = ResponseMessage::from("14\0\0");

    let sink = CapturingSink::default();
    dispatch_unsolicited_message(TEST_SERVER_VERSION, &mut message, &notice_sink_ctx(&sink));
    let notice = sink.last().expect("notice sink should receive unknown-frame notice");
    assert_eq!(notice.code, HANDSHAKE_UNKNOWN_FRAME_CODE);
    assert!(notice.is_handshake_synthetic());
}
