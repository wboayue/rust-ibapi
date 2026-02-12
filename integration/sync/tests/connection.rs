use std::sync::{Arc, Mutex};

use ibapi::client::blocking::Client;
use ibapi::messages::{IncomingMessages, ResponseMessage};
use ibapi::{ConnectionOptions, StartupMessageCallback};
use ibapi_test::{rate_limit, ClientId};

#[test]
fn connect_to_gateway() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).expect("connection failed");

    assert!(client.server_version() > 0);
    assert!(client.connection_time().is_some());

    rate_limit();
    let time = client.server_time().expect("failed to get server time");
    assert!(time.year() >= 2025);
}

#[test]
fn connect_with_callback() {
    let client_id = ClientId::get();
    let messages: Arc<Mutex<Vec<ResponseMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();

    let callback: StartupMessageCallback = Box::new(move |msg| {
        messages_clone.lock().unwrap().push(msg);
    });

    rate_limit();
    let client = Client::connect_with_callback("127.0.0.1:4002", client_id.id(), Some(callback)).expect("connection failed");

    assert!(client.server_version() > 0);

    let captured = messages.lock().unwrap();
    for msg in captured.iter() {
        assert_ne!(msg.message_type(), IncomingMessages::NotValid);
        assert!(!msg.is_empty());
    }
}

#[test]
fn connect_with_options_callback() {
    let client_id = ClientId::get();
    let messages: Arc<Mutex<Vec<ResponseMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();

    let options = ConnectionOptions::default().tcp_no_delay(true).startup_callback(move |msg| {
        messages_clone.lock().unwrap().push(msg);
    });

    rate_limit();
    let client = Client::connect_with_options("127.0.0.1:4002", client_id.id(), options).expect("connection failed");

    assert!(client.server_version() > 0);

    let captured = messages.lock().unwrap();
    for msg in captured.iter() {
        assert_ne!(msg.message_type(), IncomingMessages::NotValid);
        assert!(!msg.is_empty());
    }
}
