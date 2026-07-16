use super::*;
use crate::testdata::builders::config::config_response;
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn test_decode_config_proto_populated() {
    let bytes = config_response()
        .read_only_api(true)
        .socket_port(4002)
        .trusted_ip("127.0.0.1")
        .bypass_bond_warning(true)
        .seek_price_improvement(true)
        .auto_logoff_time("23:59")
        .message(1, "Order Warning", false)
        .encode_proto();

    let config = decode_config_proto(&bytes).unwrap();

    let api = config.api.expect("api present");
    let settings = api.settings.expect("settings present");
    assert_eq!(settings.read_only_api, Some(true));
    assert_eq!(settings.socket_port, Some(4002));
    assert_eq!(settings.trusted_ips, vec!["127.0.0.1".to_string()]);
    assert_eq!(api.precautions.expect("precautions present").bypass_bond_warning, Some(true));

    let orders = config.orders.expect("orders present");
    assert_eq!(orders.smart_routing.expect("smart routing present").seek_price_improvement, Some(true));

    assert_eq!(
        config.lock_and_exit.expect("lock_and_exit present").auto_logoff_time,
        Some("23:59".to_string())
    );

    assert_eq!(config.messages.len(), 1);
    assert_eq!(config.messages[0].id, Some(1));
    assert_eq!(config.messages[0].title, Some("Order Warning".to_string()));
    assert_eq!(config.messages[0].enabled, Some(false));
}

#[test]
fn test_decode_config_proto_empty() {
    let bytes = proto::ConfigResponse::default().encode_to_vec();

    let config = decode_config_proto(&bytes).unwrap();

    assert_eq!(config, Config::default());
    assert!(config.api.is_none());
    assert!(config.orders.is_none());
    assert!(config.lock_and_exit.is_none());
    assert!(config.messages.is_empty());
}

#[test]
fn test_decode_config_message_dispatches_config_response() {
    let bytes = config_response().read_only_api(false).encode_proto();
    let message = crate::common::test_utils::helpers::proto_response(IncomingMessages::ConfigResponse, bytes);

    let config = decode_config_message(&message).unwrap();
    assert_eq!(config.api.unwrap().settings.unwrap().read_only_api, Some(false));
}

#[test]
fn test_decode_config_message_routes_error() {
    // IncomingMessages::Error == 4; a text-framed error surfaces as an Err.
    let message = ResponseMessage::from("4\09000\0322\0error text\0");
    assert!(decode_config_message(&message).is_err());
}

#[test]
fn test_decode_config_message_rejects_unexpected_type() {
    // WshMetaData (104) is not claimed by the config decoder.
    let message = ResponseMessage::from("104\09000\0{}\0");
    match decode_config_message(&message) {
        Err(Error::UnexpectedResponse(_)) => {}
        other => panic!("expected UnexpectedResponse, got {other:?}"),
    }
}

#[test]
fn test_decode_config_rejects_text_framing() {
    // Text-framed arrival at a proto-only decoder must surface
    // UnexpectedResponse.
    let message = ResponseMessage::from("110\09000\0\0");
    match decode_config(&message) {
        Err(Error::UnexpectedResponse(_)) => {}
        other => panic!("expected UnexpectedResponse, got {other:?}"),
    }
}
