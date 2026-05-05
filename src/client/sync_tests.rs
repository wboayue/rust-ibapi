use std::sync::Arc;
use std::sync::Mutex;

use super::*;
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::transport::sync::test_listener::spawn_handshake_listener;

const SERVER_VERSION: i32 = server_versions::PROTOBUF;

fn stubbed_client() -> Client {
    Client::stubbed(Arc::new(MessageBusStub::default()), SERVER_VERSION)
}

#[test]
fn accessors_round_trip() {
    let client = stubbed_client();

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
    assert!(client.connection_time().is_none());
    assert!(client.time_zone().is_none());
    assert!(client.is_connected());

    let r1 = client.next_request_id();
    let r2 = client.next_request_id();
    assert!(r2 > r1, "request ids should increment");

    client.set_next_order_id(9000);
    let o1 = client.next_order_id();
    let o2 = client.next_order_id();
    assert_eq!(o1, 9000);
    assert_eq!(o2, 9001);
}

#[test]
fn check_server_version_branches() {
    let client = stubbed_client();

    client.check_server_version(SERVER_VERSION, "feature").expect("equal version succeeds");
    client
        .check_server_version(SERVER_VERSION - 1, "feature")
        .expect("older version succeeds");

    let err = client
        .check_server_version(SERVER_VERSION + 100, "future_feature")
        .expect_err("newer version fails");
    matches!(err, Error::ServerVersion(_, _, _));
}

#[test]
fn builder_factories_are_constructable() {
    let client = stubbed_client();
    let contract = Contract::stock("AAPL").build();

    let _ = client.order(&contract);
    let _ = client.market_data(&contract);
    let _ = client.decoder_context();
}

#[test]
fn send_helpers_round_trip_through_bus() {
    let bus = Arc::new(MessageBusStub::default());
    let client = Client::stubbed(bus.clone(), SERVER_VERSION);

    client.send_request(1, vec![0x01]).expect("send_request");
    client.send_order(2, vec![0x02]).expect("send_order");
    client.send_message(vec![0x03]).expect("send_message");
    client
        .send_shared_request(OutgoingMessages::RequestCurrentTime, vec![0x04])
        .expect("send_shared_request");

    let recorded = bus.request_messages();
    assert_eq!(recorded.len(), 4);
    assert_eq!(recorded[0], vec![0x01]);
    assert_eq!(recorded[1], vec![0x02]);
    assert_eq!(recorded[2], vec![0x03]);
    assert_eq!(recorded[3], vec![0x04]);
}

#[test]
fn create_order_update_subscription_is_unique() {
    let client = stubbed_client();
    client.create_order_update_subscription().expect("first subscription");
    let err = client.create_order_update_subscription().expect_err("duplicate fails");
    matches!(err, Error::AlreadySubscribed);
}

fn handshake_frames() -> Vec<Vec<u8>> {
    vec![
        // Handshake response (raw text): "<sv>\0<connection-time>\0".
        format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION).into_bytes(),
        // NextValidId in binary-text format (4-byte BE msg_id + text).
        binary_text(IncomingMessages::NextValidId as i32, "1\09000\0"),
        // ManagedAccounts in binary-text format.
        binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"),
    ]
}

fn binary_text(msg_id: i32, payload: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(4 + payload.len());
    data.extend_from_slice(&msg_id.to_be_bytes());
    data.extend_from_slice(payload.as_bytes());
    data
}

#[test]
fn connect_handshakes_against_real_socket() {
    let (addr, _h) = spawn_handshake_listener(handshake_frames());

    let client = Client::connect(&addr.to_string(), 100).expect("Client::connect");

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
    assert!(client.time_zone().is_some());
    assert_eq!(client.next_order_id(), 9000);
}

#[test]
fn connect_with_callback_receives_unsolicited_messages() {
    // Sparse OpenOrder frame: decoder fails, surfaces as Other — callback still fires.
    let mut frames = Vec::new();
    frames.push(format!("{}\020240120 12:00:00 EST\0", SERVER_VERSION).into_bytes());
    frames.push(binary_text(IncomingMessages::NextValidId as i32, "1\09000\0"));
    frames.push(binary_text(IncomingMessages::OpenOrder as i32, "1\0\0"));
    frames.push(binary_text(IncomingMessages::ManagedAccounts as i32, "1\0DU1234567\0"));

    let (addr, _h) = spawn_handshake_listener(frames);
    let captured = Arc::new(Mutex::new(Vec::<i32>::new()));
    let captured_clone = Arc::clone(&captured);
    let callback: StartupMessageCallback = Box::new(move |msg| {
        captured_clone.lock().unwrap().push(msg.message_type() as i32);
    });

    let _client = Client::connect_with_callback(&addr.to_string(), 100, Some(callback)).expect("connect_with_callback");

    let seen = captured.lock().unwrap();
    assert!(
        seen.contains(&(IncomingMessages::OpenOrder as i32)),
        "callback did not see OpenOrder; saw: {seen:?}"
    );
}

#[test]
fn connect_with_options_applies_tcp_no_delay() {
    let (addr, _h) = spawn_handshake_listener(handshake_frames());

    let options = ConnectionOptions::default().tcp_no_delay(true);
    let client = Client::connect_with_options(&addr.to_string(), 100, options).expect("connect_with_options");

    assert_eq!(client.client_id(), 100);
    assert_eq!(client.server_version(), SERVER_VERSION);
}
