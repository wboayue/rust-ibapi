use super::*;
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_CONTRACT_ID, TEST_TICKER_ID};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::testdata::builders::{RequestEncoder, ResponseEncoder};
use prost::Message;

fn split_pipe(s: &str) -> Vec<&str> {
    s.split_terminator('|').collect()
}

#[test]
fn position_default_serializes_all_fields_in_order() {
    let encoded = position().encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[0], "61");
    assert_eq!(fields[1], "3");
    assert_eq!(fields[2], TEST_ACCOUNT);
    assert_eq!(fields[3], TEST_CONTRACT_ID.to_string());
    assert_eq!(fields[4], "TSLA");
    assert_eq!(fields[5], "STK");
    assert_eq!(fields[6], "");
    assert_eq!(fields[7], "0");
    assert_eq!(fields[8], "");
    assert_eq!(fields[9], "");
    assert_eq!(fields[10], "NASDAQ");
    assert_eq!(fields[11], "USD");
    assert_eq!(fields[12], "TSLA");
    assert_eq!(fields[13], "NMS");
    assert_eq!(fields[14], "500");
    assert_eq!(fields[15], "196.77");
    assert_eq!(fields.len(), 16);
}

#[test]
fn position_setters_override_defaults() {
    let encoded = position()
        .symbol("AAPL")
        .contract_id(265598)
        .position(100.0)
        .average_cost(150.25)
        .encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[3], "265598");
    assert_eq!(fields[4], "AAPL");
    assert_eq!(fields[14], "100");
    assert_eq!(fields[15], "150.25");
}

#[test]
fn position_encode_null_uses_nul_separator() {
    let nul = position().encode_null();
    assert!(nul.starts_with("61\03\0"));
    assert!(nul.ends_with('\0'));
    assert!(!nul.contains('|'));
}

#[test]
fn position_encode_length_prefixed_wraps_null_payload() {
    let bytes = position().encode_length_prefixed();
    let payload_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    assert_eq!(payload_len, bytes.len() - 4);
    assert_eq!(&bytes[4..], position().encode_null().as_bytes());
}

#[test]
fn position_end_default_emits_header_only() {
    assert_eq!(position_end().encode_pipe(), "62|1|");
}

#[test]
fn position_multi_default_serializes_all_fields_in_order() {
    let encoded = position_multi().encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[0], "71");
    assert_eq!(fields[1], "3");
    assert_eq!(fields[2], TEST_TICKER_ID.to_string());
    assert_eq!(fields[3], TEST_ACCOUNT);
    assert_eq!(fields[4], TEST_CONTRACT_ID.to_string());
    assert_eq!(fields[5], "TSLA");
    assert_eq!(fields[6], "STK");
    assert_eq!(fields[13], "TSLA");
    assert_eq!(fields[14], "NMS");
    assert_eq!(fields[15], "500");
    assert_eq!(fields[16], "196.77");
    assert_eq!(fields[17], "");
    assert_eq!(fields.len(), 18);
}

#[test]
fn position_multi_setters_override_defaults() {
    let encoded = position_multi().request_id(42).symbol("MSFT").model_code("TARGET2024").encode_pipe();
    let fields = split_pipe(&encoded);

    assert_eq!(fields[2], "42");
    assert_eq!(fields[5], "MSFT");
    assert_eq!(fields[17], "TARGET2024");
}

#[test]
fn position_multi_end_default_uses_test_ticker_id() {
    assert_eq!(position_multi_end().encode_pipe(), format!("72|1|{}|", TEST_TICKER_ID));
}

#[test]
fn position_multi_end_request_id_setter() {
    assert_eq!(position_multi_end().request_id(7).encode_pipe(), "72|1|7|");
}

// === Protobuf encode/decode round-trips ===

#[test]
fn position_proto_round_trips_default_fields() {
    let bytes = position().encode_proto();

    let decoded = proto::Position::decode(&bytes[..]).unwrap();

    assert_eq!(decoded.account.as_deref(), Some(TEST_ACCOUNT));
    assert_eq!(decoded.position.as_deref(), Some("500"));
    assert_eq!(decoded.avg_cost, Some(196.77));

    let contract = decoded.contract.as_ref().unwrap();
    assert_eq!(contract.con_id, Some(TEST_CONTRACT_ID));
    assert_eq!(contract.symbol.as_deref(), Some("TSLA"));
    assert_eq!(contract.sec_type.as_deref(), Some("STK"));
    assert_eq!(contract.exchange.as_deref(), Some("NASDAQ"));
    assert_eq!(contract.currency.as_deref(), Some("USD"));
    assert_eq!(contract.local_symbol.as_deref(), Some("TSLA"));
    assert_eq!(contract.trading_class.as_deref(), Some("NMS"));
    assert_eq!(contract.last_trade_date_or_contract_month, None);
    assert_eq!(contract.strike, None);
    assert_eq!(contract.right, None);
    assert_eq!(contract.multiplier, None);
}

#[test]
fn position_proto_round_trips_setter_overrides() {
    let bytes = position()
        .symbol("AAPL")
        .contract_id(265598)
        .position(150.0)
        .average_cost(99.5)
        .encode_proto();

    let decoded = proto::Position::decode(&bytes[..]).unwrap();

    let contract = decoded.contract.as_ref().unwrap();
    assert_eq!(contract.con_id, Some(265598));
    assert_eq!(contract.symbol.as_deref(), Some("AAPL"));
    assert_eq!(decoded.position.as_deref(), Some("150"));
    assert_eq!(decoded.avg_cost, Some(99.5));
}

#[test]
fn position_to_proto_matches_encode_proto_bytes() {
    let builder = position().symbol("MSFT").position(42.0);

    let direct = {
        let mut bytes = Vec::new();
        builder.to_proto().encode(&mut bytes).unwrap();
        bytes
    };

    assert_eq!(direct, builder.encode_proto());
}

#[test]
fn position_end_proto_is_empty() {
    let bytes = position_end().encode_proto();
    assert!(bytes.is_empty(), "PositionEnd has no fields, so encoded form is zero-length");

    let decoded = proto::PositionEnd::decode(&bytes[..]).unwrap();
    assert_eq!(decoded, proto::PositionEnd {});
}

#[test]
fn position_multi_proto_round_trips_default_fields() {
    let bytes = position_multi().encode_proto();

    let decoded = proto::PositionMulti::decode(&bytes[..]).unwrap();

    assert_eq!(decoded.req_id, Some(TEST_TICKER_ID));
    assert_eq!(decoded.account.as_deref(), Some(TEST_ACCOUNT));
    assert_eq!(decoded.position.as_deref(), Some("500"));
    assert_eq!(decoded.avg_cost, Some(196.77));
    assert_eq!(decoded.model_code, None);

    let contract = decoded.contract.as_ref().unwrap();
    assert_eq!(contract.con_id, Some(TEST_CONTRACT_ID));
    assert_eq!(contract.symbol.as_deref(), Some("TSLA"));
    assert_eq!(contract.trading_class.as_deref(), Some("NMS"));
}

#[test]
fn position_multi_proto_round_trips_setter_overrides() {
    let bytes = position_multi().request_id(42).symbol("MSFT").model_code("TARGET2024").encode_proto();

    let decoded = proto::PositionMulti::decode(&bytes[..]).unwrap();

    assert_eq!(decoded.req_id, Some(42));
    assert_eq!(decoded.model_code.as_deref(), Some("TARGET2024"));
    assert_eq!(decoded.contract.as_ref().unwrap().symbol.as_deref(), Some("MSFT"));
}

#[test]
fn position_multi_end_proto_round_trips_request_id() {
    let bytes = position_multi_end().request_id(7).encode_proto();

    let decoded = proto::PositionMultiEnd::decode(&bytes[..]).unwrap();
    assert_eq!(decoded.req_id, Some(7));
}

// === Request builders ===

fn split_msg_header(bytes: &[u8]) -> (i32, &[u8]) {
    const PROTOBUF_MSG_ID_OFFSET: i32 = 200;
    let raw = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    (raw - PROTOBUF_MSG_ID_OFFSET, &bytes[4..])
}

#[test]
fn request_positions_encodes_correct_msg_id_and_empty_body() {
    let bytes = request_positions().encode_request();

    let (msg_id, body) = split_msg_header(&bytes);
    assert_eq!(msg_id, OutgoingMessages::RequestPositions as i32);
    assert!(body.is_empty(), "RequestPositions has no fields");
}

#[test]
fn cancel_positions_encodes_correct_msg_id_and_empty_body() {
    let bytes = cancel_positions().encode_request();

    let (msg_id, body) = split_msg_header(&bytes);
    assert_eq!(msg_id, OutgoingMessages::CancelPositions as i32);
    assert!(body.is_empty());
}

#[test]
fn request_positions_multi_round_trips_default_fields() {
    let bytes = request_positions_multi().encode_request();
    let (msg_id, body) = split_msg_header(&bytes);
    assert_eq!(msg_id, OutgoingMessages::RequestPositionsMulti as i32);

    let decoded = proto::PositionsMultiRequest::decode(body).unwrap();
    assert_eq!(decoded.req_id, Some(TEST_TICKER_ID));
    assert_eq!(decoded.account.as_deref(), Some(TEST_ACCOUNT));
    assert_eq!(decoded.model_code, None);
}

#[test]
fn request_positions_multi_setters_override_defaults() {
    let bytes = request_positions_multi()
        .request_id(42)
        .account("DU7654321")
        .model_code("TARGET2024")
        .encode_request();
    let (_, body) = split_msg_header(&bytes);

    let decoded = proto::PositionsMultiRequest::decode(body).unwrap();
    assert_eq!(decoded.req_id, Some(42));
    assert_eq!(decoded.account.as_deref(), Some("DU7654321"));
    assert_eq!(decoded.model_code.as_deref(), Some("TARGET2024"));
}

#[test]
fn cancel_positions_multi_round_trips_request_id() {
    let bytes = cancel_positions_multi().request_id(99).encode_request();
    let (msg_id, body) = split_msg_header(&bytes);
    assert_eq!(msg_id, OutgoingMessages::CancelPositionsMulti as i32);

    let decoded = proto::CancelPositionsMulti::decode(body).unwrap();
    assert_eq!(decoded.req_id, Some(99));
}

#[test]
fn request_encoder_encode_proto_omits_msg_id_header() {
    let builder = request_positions_multi().account("X");
    let proto_only = builder.encode_proto();
    let with_header = builder.encode_request();

    assert_eq!(with_header.len(), proto_only.len() + 4);
    assert_eq!(&with_header[4..], &proto_only[..]);
}
