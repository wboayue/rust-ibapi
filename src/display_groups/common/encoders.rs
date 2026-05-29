//! Encoders for display group messages.

use crate::messages::OutgoingMessages;
use crate::Error;

#[allow(dead_code)]
pub(crate) fn encode_query_display_groups(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, QueryDisplayGroupsRequest, OutgoingMessages::QueryDisplayGroups)
}

pub(crate) fn encode_subscribe_to_group_events(request_id: i32, group_id: i32) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::SubscribeToGroupEventsRequest {
        req_id: Some(request_id),
        group_id: Some(group_id),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::SubscribeToGroupEvents as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_update_display_group(request_id: i32, contract_info: &str) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::UpdateDisplayGroupRequest {
        req_id: Some(request_id),
        contract_info: Some(contract_info.to_string()),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::UpdateDisplayGroup as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_unsubscribe_from_group_events(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(
        request_id,
        UnsubscribeFromGroupEventsRequest,
        OutgoingMessages::UnsubscribeFromGroupEvents
    )
}

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;
