use crate::client::r#async::Client;
use crate::common::test_utils::helpers::{decode_request_proto, proto_response, TEST_REQ_ID_FIRST};
use crate::config::{ApiConfig, ApiSettings, ConfigWarning, MessageSetting, OrdersConfig, OrdersSmartRouting};
use crate::messages::IncomingMessages;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::config::update_config_response;
use crate::testdata::builders::ResponseProtoEncoder;
use crate::{proto, Error};
use std::sync::Arc;

fn stub(responses: Vec<crate::messages::ResponseMessage>) -> Arc<MessageBusStub> {
    Arc::new(MessageBusStub::with_ordered_responses(responses))
}

#[tokio::test]
async fn test_update_config_request_body() {
    let message_bus = stub(vec![proto_response(
        IncomingMessages::UpdateConfigResponse,
        update_config_response().request_id(TEST_REQ_ID_FIRST).status("ok").encode_proto(),
    )]);

    let client = Client::stubbed(message_bus.clone(), crate::server_versions::UPDATE_CONFIG);
    client
        .update_config()
        .api(ApiConfig {
            settings: Some(ApiSettings {
                socket_port: Some(7497),
                ..Default::default()
            }),
            ..Default::default()
        })
        .orders(OrdersConfig {
            smart_routing: Some(OrdersSmartRouting {
                seek_price_improvement: Some(true),
                ..Default::default()
            }),
        })
        .message(MessageSetting {
            id: Some(131),
            enabled: Some(false),
            ..Default::default()
        })
        .accept_warning(ConfigWarning {
            message_id: Some(131),
            ..Default::default()
        })
        .reset_api_order_sequence()
        .submit()
        .await
        .expect("update config failed");

    let sent: proto::UpdateConfigRequest = decode_request_proto(&message_bus, 0);
    assert_eq!(sent.req_id, Some(TEST_REQ_ID_FIRST));
    assert_eq!(sent.api.unwrap().settings.unwrap().socket_port, Some(7497));
    assert_eq!(sent.orders.unwrap().smart_routing.unwrap().seek_price_improvement, Some(true));
    assert_eq!(sent.messages.len(), 1);
    assert_eq!(sent.accepted_warnings.len(), 1);
    assert_eq!(sent.reset_api_order_sequence, Some(true));
}

#[tokio::test]
async fn test_update_config_response_decoded() {
    let message_bus = stub(vec![proto_response(
        IncomingMessages::UpdateConfigResponse,
        update_config_response()
            .request_id(TEST_REQ_ID_FIRST)
            .status("warning")
            .warning(131, "Confirm Mandatory Cap Price")
            .encode_proto(),
    )]);

    let client = Client::stubbed(message_bus, crate::server_versions::UPDATE_CONFIG);
    let response = client
        .update_config()
        .reset_api_order_sequence()
        .submit()
        .await
        .expect("update config failed");

    assert_eq!(response.status.as_deref(), Some("warning"));
    assert_eq!(response.warnings.len(), 1);
    assert_eq!(response.warnings[0].message_id, Some(131));
}

#[tokio::test]
async fn test_update_config_server_version_error() {
    let message_bus = stub(vec![]);

    let client = Client::stubbed(message_bus, crate::server_versions::UPDATE_CONFIG - 1);
    match client.update_config().reset_api_order_sequence().submit().await {
        Err(Error::ServerVersion(_, _, _)) => {}
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_update_config_unexpected_end_of_stream() {
    let message_bus = stub(vec![]);

    let client = Client::stubbed(message_bus, crate::server_versions::UPDATE_CONFIG);
    match client.update_config().reset_api_order_sequence().submit().await {
        Err(Error::UnexpectedEndOfStream) => {}
        other => panic!("expected UnexpectedEndOfStream, got {other:?}"),
    }
}
