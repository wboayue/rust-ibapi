use super::*;
use crate::server_versions;

#[test]
fn refresh_applies_the_reconnect_handshake() {
    let state = HandshakeState {
        order_ids: Arc::new(ClientIdManager::new(100).unwrap()),
        server_version: Arc::new(ServerVersion::new(server_versions::PROTOBUF_REST_MESSAGES_3)),
    };
    let metadata = ConnectionMetadata {
        next_order_id: 5000,
        server_version: server_versions::CONDITIONS_INCLUDE_OVERNIGHT,
        ..Default::default()
    };

    state.refresh(&metadata);

    assert_eq!(state.order_ids.current_order_id(), 5000);
    assert_eq!(state.server_version.get(), server_versions::CONDITIONS_INCLUDE_OVERNIGHT);
}
