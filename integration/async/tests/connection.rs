use std::sync::{Arc, Mutex};

use ibapi::messages::{IncomingMessages, ResponseMessage};
use ibapi::{Client, ConnectionOptions, StartupMessageCallback};
use ibapi_test::ClientId;

#[tokio::test]
async fn connect_to_gateway() {
    let client_id = ClientId::get();
    let client = Client::connect("127.0.0.1:4002", client_id.id()).await.expect("connection failed");

    assert!(client.server_version() > 0);
    assert!(client.connection_time().is_some());

    let time = client.server_time().await.expect("failed to get server time");
    assert!(time.year() >= 2025);
}

#[tokio::test]
async fn connect_with_callback() {
    let client_id = ClientId::get();
    let messages: Arc<Mutex<Vec<ResponseMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();

    let callback: StartupMessageCallback = Box::new(move |msg| {
        messages_clone.lock().unwrap().push(msg);
    });

    let client = Client::connect_with_callback("127.0.0.1:4002", client_id.id(), Some(callback))
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);

    let captured = messages.lock().unwrap();
    for msg in captured.iter() {
        assert_ne!(msg.message_type(), IncomingMessages::NotValid);
        assert!(!msg.is_empty());
    }
}

#[tokio::test]
async fn connect_with_options_callback() {
    let client_id = ClientId::get();
    let messages: Arc<Mutex<Vec<ResponseMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();

    let options = ConnectionOptions::default().tcp_no_delay(true).startup_callback(move |msg| {
        messages_clone.lock().unwrap().push(msg);
    });

    let client = Client::connect_with_options("127.0.0.1:4002", client_id.id(), options)
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);

    let captured = messages.lock().unwrap();
    for msg in captured.iter() {
        assert_ne!(msg.message_type(), IncomingMessages::NotValid);
        assert!(!msg.is_empty());
    }
}
