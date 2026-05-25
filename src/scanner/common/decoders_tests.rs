use super::*;
use crate::messages::ResponseMessage;
use crate::server_versions;

#[test]
fn test_decode_scanner_data_proto() {
    use prost::Message;

    let proto_msg = crate::proto::ScannerData {
        req_id: Some(1),
        scanner_data_element: vec![
            crate::proto::ScannerDataElement {
                rank: Some(0),
                contract: Some(crate::proto::Contract {
                    con_id: Some(265598),
                    symbol: Some("AAPL".into()),
                    sec_type: Some("STK".into()),
                    ..Default::default()
                }),
                market_name: Some("NMS".into()),
                distance: Some("1.5".into()),
                benchmark: Some("".into()),
                projection: Some("".into()),
                combo_key: Some("".into()),
            },
            crate::proto::ScannerDataElement {
                rank: Some(1),
                contract: Some(crate::proto::Contract {
                    con_id: Some(76792991),
                    symbol: Some("TSLA".into()),
                    sec_type: Some("STK".into()),
                    ..Default::default()
                }),
                market_name: Some("NMS".into()),
                distance: None,
                benchmark: None,
                projection: None,
                combo_key: None,
            },
        ],
    };

    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let results = decode_scanner_data_proto(&bytes).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].rank, 0);
    assert_eq!(results[0].contract_details.contract.contract_id, 265598);
    assert_eq!(results[0].contract_details.market_name, "NMS");
}

#[test]
fn test_decode_scanner_parameters_proto() {
    use prost::Message;
    let xml = "<ScanParameterResponse><ScanTypeList /></ScanParameterResponse>";
    let proto_msg = crate::proto::ScannerParameters { xml: Some(xml.into()) };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_scanner_parameters_proto(&bytes).unwrap();
    assert_eq!(result, xml);
}

#[test]
fn test_decode_scanner_parameters_proto_empty() {
    use prost::Message;
    let proto_msg = crate::proto::ScannerParameters { xml: None };
    let mut bytes = Vec::new();
    proto_msg.encode(&mut bytes).unwrap();

    let result = decode_scanner_parameters_proto(&bytes).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_decode_scanner_data_rejects_text_framing() {
    // Servers ≥ the connection floor always emit ScannerData in proto.
    // Text-framed arrival skip-classifies via `UnexpectedResponse` (rule 20)
    // rather than terminating the subscription.
    let message = ResponseMessage::from("20\03\09000\01\00\0265598\0AAPL\0STK\0\00\0\0SMART\0USD\0AAPL\0NMS\0NMS\0\0\0\0\0")
        .with_server_version(server_versions::PROTOBUF_REST_MESSAGES_3);
    let err = decode_scanner_data(&message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}

#[test]
fn test_decode_scanner_parameters_rejects_text_framing() {
    let message = ResponseMessage::from("19\02\0<ScanParameterResponse/>\0").with_server_version(server_versions::PROTOBUF_REST_MESSAGES_3);
    let err = decode_scanner_parameters(&message).expect_err("text framing must be rejected");
    assert!(
        matches!(err, Error::UnexpectedResponse(_)),
        "expected Error::UnexpectedResponse, got {err:?}"
    );
}
