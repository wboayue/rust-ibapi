use std::sync::{Arc, Mutex};

use serial_test::serial;

use super::*;
use crate::client::ids::{OrderId, RequestId};
use crate::common::test_utils::helpers::{
    binary_text, error_frame, handshake_frames, handshake_response_frame, managed_accounts_frame, next_valid_id_frame, TEST_ACCOUNT,
    TEST_ORDER_ID_SEED,
};
use crate::messages::{encode_raw_length, IncomingMessages, OutgoingMessages};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::transport::r#async::test_listener::spawn_handshake_listener;
use crate::transport::raw_capture::test_support;

const SERVER_VERSION: i32 = server_versions::PROTOBUF_REST_MESSAGES_3;

fn stubbed_client() -> Client {
    Client::stubbed(Arc::new(MessageBusStub::default()), SERVER_VERSION)
}

#[tokio::test]
async fn accessors_round_trip() {
    let client = stubbed_client();

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
    assert!(client.connection_time().is_none());
    assert!(client.time_zone().is_none());
    assert!(client.is_connected());

    let r1 = client.mint_request_id();
    let r2 = client.mint_request_id();
    assert!(r2 > r1, "request ids should increment");

    client.raise_next_order_id(OrderId::from(9000));
    let o1 = client.next_order_id();
    let o2 = client.next_order_id();
    assert_eq!(o1, 9000);
    assert_eq!(o2, 9001);
}

#[tokio::test]
async fn check_server_version_branches() {
    let client = stubbed_client();

    client.check_server_version(SERVER_VERSION, "feature").expect("equal version succeeds");
    client
        .check_server_version(SERVER_VERSION - 1, "feature")
        .expect("older version succeeds");

    let err = client
        .check_server_version(SERVER_VERSION + 100, "future_feature")
        .expect_err("newer version fails");
    assert!(matches!(err, Error::ServerVersion(_, _, _)), "got {err:?}");
}

#[tokio::test]
async fn decoder_context_is_constructable() {
    let client = stubbed_client();

    let _ = client.decoder_context();
}

#[tokio::test]
async fn send_helpers_round_trip_through_bus() {
    let bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(bus.clone(), SERVER_VERSION);

    client.send_request(RequestId::nth(1), vec![0x01]).await.expect("send_request");
    client.send_order(OrderId::from(2), vec![0x02]).await.expect("send_order");
    client.send_message(vec![0x03]).await.expect("send_message");
    client
        .send_shared_request(OutgoingMessages::RequestCurrentTime, vec![0x04])
        .await
        .expect("send_shared_request");

    let recorded = bus.request_messages();
    assert_eq!(recorded.len(), 4);
    assert_eq!(recorded[0], vec![0x01]);
    assert_eq!(recorded[1], vec![0x02]);
    assert_eq!(recorded[2], vec![0x03]);
    assert_eq!(recorded[3], vec![0x04]);
}

#[tokio::test]
async fn create_order_update_subscription_is_unique() {
    let client = stubbed_client();
    let _first = client.create_order_update_subscription().await.expect("first subscription");
    let err = client.create_order_update_subscription().await.err().expect("duplicate fails");
    assert!(matches!(err, Error::AlreadySubscribed), "got {err:?}");
}

fn default_handshake_frames() -> Vec<Vec<u8>> {
    handshake_frames(SERVER_VERSION, "EST", TEST_ORDER_ID_SEED)
}

/// A server whose next valid order id is already in the request range is
/// refused at connect: every order the session placed would collide with
/// request ids (#789).
#[tokio::test]
async fn connect_rejects_next_valid_id_in_request_range() {
    let mut frames = default_handshake_frames();
    frames[1] = next_valid_id_frame(crate::client::ids::REQUEST_ID_FLOOR);
    let (addr, _h) = spawn_handshake_listener(frames).await;

    match Client::connect(&addr.to_string(), 100).await {
        Err(Error::ConnectionRejected(message)) => assert!(message.contains("next valid order id"), "{message}"),
        Err(other) => panic!("expected ConnectionRejected, got {other:?}"),
        Ok(_) => panic!("connect accepted a next valid order id in the request range"),
    }
}

#[tokio::test]
async fn connect_handshakes_against_real_socket() {
    let (addr, _h) = spawn_handshake_listener(default_handshake_frames()).await;

    let client = Client::connect(&addr.to_string(), 100).await.expect("Client::connect");

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
    assert!(client.time_zone().is_some());
    assert_eq!(client.next_order_id(), 9000);
}

/// Async twin of `client::sync_tests::raw_capture_env_var_records_framed_wire_bytes`.
/// `AsyncTcpSocket` builds its own tap, so the blocking test says nothing about
/// this path.
#[tokio::test]
#[serial]
async fn raw_capture_env_var_records_framed_wire_bytes() {
    let frames = default_handshake_frames();
    let (addr, _h) = spawn_handshake_listener(frames.clone()).await;
    let dir = tempfile::TempDir::new().unwrap();

    temp_env::async_with_vars([("IBAPI_RAW_CAPTURE_DIR", Some(dir.path().to_str().unwrap()))], async {
        let client = Client::connect(&addr.to_string(), 100).await.expect("Client::connect");
        assert_eq!(client.server_version(), SERVER_VERSION);

        let capture = test_support::frames(dir.path());
        // Anything past the handshake races the dispatcher task, so this asserts
        // a prefix of the capture rather than the whole of it.
        assert!(
            capture.starts_with(&encode_raw_length(&frames[0])),
            "capture must begin with the framed handshake response, got {:?}",
            &capture[..capture.len().min(32)]
        );
        assert!(
            capture.windows(frames[1].len()).any(|window| window == frames[1]),
            "capture must contain the next-valid-id frame"
        );
    })
    .await;
}

#[tokio::test]
async fn builder_startup_callback_receives_unsolicited_messages() {
    // OpenOrderEnd (msg=53) is a unit marker — no payload to decode, so the
    // typed callback fires regardless of wire framing. Sparse OpenOrder /
    // OrderStatus frames would fail the proto-only decoder and route to the
    // notice stream instead.
    let frames = vec![
        handshake_response_frame(SERVER_VERSION, "EST"),
        next_valid_id_frame(TEST_ORDER_ID_SEED),
        binary_text(IncomingMessages::OpenOrderEnd as i32, "1\0"),
        managed_accounts_frame(TEST_ACCOUNT),
    ];

    let (addr, _h) = spawn_handshake_listener(frames).await;
    let captured = Arc::new(Mutex::new(Vec::<i32>::new()));
    let captured_clone = Arc::clone(&captured);

    let _client = Client::builder()
        .address(addr.to_string())
        .client_id(100)
        .startup_callback(move |msg| {
            captured_clone.lock().unwrap().push(msg.message_type() as i32);
        })
        .connect()
        .await
        .expect("ClientBuilder::connect");

    let seen = captured.lock().unwrap();
    assert!(
        seen.contains(&(IncomingMessages::OpenOrderEnd as i32)),
        "callback did not see OpenOrderEnd; saw: {seen:?}"
    );
}

#[tokio::test]
async fn builder_tcp_no_delay_round_trips() {
    let (addr, _h) = spawn_handshake_listener(default_handshake_frames()).await;

    let client = Client::builder()
        .address(addr.to_string())
        .client_id(100)
        .tcp_no_delay(false)
        .connect()
        .await
        .expect("ClientBuilder::connect");

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
}

#[tokio::test]
async fn builder_connect_with_notice_stream_captures_handshake_notice() {
    // Frames: handshake + farm-status notice during account-info phase + ManagedAccounts + NextValidId.
    let frames = vec![
        handshake_response_frame(SERVER_VERSION, "EST"),
        next_valid_id_frame(TEST_ORDER_ID_SEED),
        error_frame(-1, 2104, "farm OK"),
        managed_accounts_frame(TEST_ACCOUNT),
    ];

    let (addr, _h) = spawn_handshake_listener(frames).await;

    let (_client, mut notices) = Client::builder()
        .address(addr.to_string())
        .client_id(100)
        .connect_with_notice_stream()
        .await
        .expect("connect_with_notice_stream");

    let n = tokio::time::timeout(std::time::Duration::from_secs(2), notices.next())
        .await
        .expect("timed out waiting for handshake notice")
        .expect("notice stream closed");
    assert_eq!(n.code, 2104);
}
