use super::*;
use crate::common::test_utils::helpers::{assert_request, proto_response, TEST_REQ_ID_FIRST};
use crate::messages::IncomingMessages;
use crate::stubs::MessageBusStub;
use crate::testdata::builders::config::{config_request, config_response};
use crate::testdata::builders::ResponseProtoEncoder;
use std::sync::Arc;

fn stub(responses: Vec<crate::messages::ResponseMessage>) -> Arc<MessageBusStub> {
    Arc::new(MessageBusStub::with_ordered_responses(responses))
}

#[test]
fn test_config_request_body() {
    let message_bus = stub(vec![proto_response(
        IncomingMessages::ConfigResponse,
        config_response().request_id(TEST_REQ_ID_FIRST).encode_proto(),
    )]);

    let client = Client::stubbed(message_bus.clone(), crate::server_versions::CONFIG);
    client.config().expect("config request failed");

    assert_request(&message_bus, 0, &config_request().request_id(TEST_REQ_ID_FIRST));
}

#[test]
fn test_config_round_trip() {
    let message_bus = stub(vec![proto_response(
        IncomingMessages::ConfigResponse,
        config_response()
            .request_id(TEST_REQ_ID_FIRST)
            .read_only_api(true)
            .socket_port(4002)
            .seek_price_improvement(true)
            .encode_proto(),
    )]);

    let client = Client::stubbed(message_bus, crate::server_versions::CONFIG);
    let config = client.config().expect("config request failed");

    assert_eq!(config.api.unwrap().settings.unwrap().socket_port, Some(4002));
    assert_eq!(config.orders.unwrap().smart_routing.unwrap().seek_price_improvement, Some(true));
}

#[test]
fn test_config_unexpected_end_of_stream() {
    let message_bus = stub(vec![]);

    let client = Client::stubbed(message_bus, crate::server_versions::CONFIG);
    match client.config() {
        Err(Error::UnexpectedEndOfStream) => {}
        other => panic!("expected UnexpectedEndOfStream, got {other:?}"),
    }
}

#[test]
fn test_config_server_version_error() {
    let message_bus = stub(vec![]);

    let client = Client::stubbed(message_bus, crate::server_versions::CONFIG - 1);
    match client.config() {
        Err(Error::ServerVersion(_, _, _)) => {}
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
}
