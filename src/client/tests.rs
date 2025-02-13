use std::sync::Arc;

use crate::{stubs::MessageBusStub, transport::ConnectionMetadata};

use super::Client;

#[test]
fn test_client_id() {
    let client_id = 500;
    let connection_metadata = ConnectionMetadata {
        client_id,
        ..ConnectionMetadata::default()
    };
    let message_bus = Arc::new(MessageBusStub::default());

    let client = Client::new(connection_metadata, message_bus).unwrap();

    assert_eq!(client.client_id(), client_id);
}
