//! Encoders for Wall Street Horizon messages

use time::Date;

use crate::messages::OutgoingMessages;
use crate::wsh::AutoFill;
use crate::Error;

pub(in crate::wsh) fn encode_request_wsh_metadata(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, WshMetaDataRequest, OutgoingMessages::RequestWshMetaData)
}

pub(in crate::wsh) fn encode_cancel_wsh_metadata(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelWshMetaData, OutgoingMessages::CancelWshMetaData)
}

pub(in crate::wsh) fn encode_request_wsh_event_data(
    request_id: i32,
    contract_id: Option<i32>,
    filter: Option<&str>,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let format = time::format_description::parse("[year][month][day]").unwrap();
    let request = crate::proto::WshEventDataRequest {
        req_id: Some(request_id),
        con_id: contract_id,
        filter: filter.map(|s| s.to_string()),
        fill_watchlist: auto_fill.as_ref().map(|af| af.watchlist),
        fill_portfolio: auto_fill.as_ref().map(|af| af.portfolio),
        fill_competitors: auto_fill.as_ref().map(|af| af.competitors),
        start_date: start_date.and_then(|d| d.format(&format).ok()),
        end_date: end_date.and_then(|d| d.format(&format).ok()),
        total_limit: limit,
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestWshEventData as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::wsh) fn encode_cancel_wsh_event_data(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelWshEventData, OutgoingMessages::CancelWshEventData)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_proto_msg_id;

    #[test]
    fn test_encode_request_wsh_metadata() {
        let bytes = encode_request_wsh_metadata(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestWshMetaData);
    }

    #[test]
    fn test_encode_cancel_wsh_metadata() {
        let bytes = encode_cancel_wsh_metadata(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelWshMetaData);
    }

    #[test]
    fn test_encode_request_wsh_event_data() {
        let bytes = encode_request_wsh_event_data(
            9000,
            Some(12345),
            Some("earnings"),
            None,
            None,
            Some(10),
            Some(AutoFill {
                watchlist: true,
                portfolio: false,
                competitors: true,
            }),
        )
        .unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestWshEventData);
        use prost::Message;
        let req = crate::proto::WshEventDataRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(req.con_id, Some(12345));
        assert_eq!(req.filter.as_deref(), Some("earnings"));
        assert_eq!(req.fill_watchlist, Some(true));
    }

    #[test]
    fn test_encode_cancel_wsh_event_data() {
        let bytes = encode_cancel_wsh_event_data(9000).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelWshEventData);
    }
}
