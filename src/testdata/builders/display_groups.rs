//! Builders for display-groups response and request messages.

use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::testdata::builders::{RequestEncoder, ResponseProtoEncoder};

single_req_id_request_builder!(
    QueryDisplayGroupsRequestBuilder,
    QueryDisplayGroupsRequest,
    OutgoingMessages::QueryDisplayGroups
);

single_req_id_request_builder!(
    UnsubscribeFromGroupEventsRequestBuilder,
    UnsubscribeFromGroupEventsRequest,
    OutgoingMessages::UnsubscribeFromGroupEvents
);

#[derive(Clone, Copy, Debug)]
pub struct SubscribeToGroupEventsRequestBuilder {
    pub request_id: i32,
    pub group_id: i32,
}

impl Default for SubscribeToGroupEventsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            group_id: 1,
        }
    }
}

impl SubscribeToGroupEventsRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }

    pub fn group_id(mut self, v: i32) -> Self {
        self.group_id = v;
        self
    }
}

impl RequestEncoder for SubscribeToGroupEventsRequestBuilder {
    type Proto = proto::SubscribeToGroupEventsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::SubscribeToGroupEvents;

    fn to_proto(&self) -> Self::Proto {
        proto::SubscribeToGroupEventsRequest {
            req_id: Some(self.request_id),
            group_id: Some(self.group_id),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateDisplayGroupRequestBuilder {
    pub request_id: i32,
    pub contract_info: String,
}

impl Default for UpdateDisplayGroupRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract_info: "265598@SMART".to_string(),
        }
    }
}

impl UpdateDisplayGroupRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }

    pub fn contract_info(mut self, v: impl Into<String>) -> Self {
        self.contract_info = v.into();
        self
    }
}

impl RequestEncoder for UpdateDisplayGroupRequestBuilder {
    type Proto = proto::UpdateDisplayGroupRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::UpdateDisplayGroup;

    fn to_proto(&self) -> Self::Proto {
        proto::UpdateDisplayGroupRequest {
            req_id: Some(self.request_id),
            contract_info: Some(self.contract_info.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DisplayGroupUpdatedResponse {
    pub request_id: i32,
    pub contract_info: String,
}

impl Default for DisplayGroupUpdatedResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract_info: String::new(),
        }
    }
}

impl DisplayGroupUpdatedResponse {
    pub fn contract_info(mut self, v: impl Into<String>) -> Self {
        self.contract_info = v.into();
        self
    }
}

impl ResponseProtoEncoder for DisplayGroupUpdatedResponse {
    type Proto = proto::DisplayGroupUpdated;

    fn to_proto(&self) -> Self::Proto {
        proto::DisplayGroupUpdated {
            req_id: Some(self.request_id),
            contract_info: Some(self.contract_info.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DisplayGroupListResponse {
    pub request_id: i32,
    pub groups: String,
}

impl Default for DisplayGroupListResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            groups: String::new(),
        }
    }
}

impl DisplayGroupListResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn groups(mut self, v: impl Into<String>) -> Self {
        self.groups = v.into();
        self
    }
}

impl ResponseProtoEncoder for DisplayGroupListResponse {
    type Proto = proto::DisplayGroupList;

    fn to_proto(&self) -> Self::Proto {
        proto::DisplayGroupList {
            req_id: Some(self.request_id),
            groups: Some(self.groups.clone()),
        }
    }
}

pub fn display_group_updated() -> DisplayGroupUpdatedResponse {
    DisplayGroupUpdatedResponse::default()
}

pub fn display_group_list() -> DisplayGroupListResponse {
    DisplayGroupListResponse::default()
}

pub fn query_display_groups_request() -> QueryDisplayGroupsRequestBuilder {
    QueryDisplayGroupsRequestBuilder::default()
}

pub fn subscribe_to_group_events_request() -> SubscribeToGroupEventsRequestBuilder {
    SubscribeToGroupEventsRequestBuilder::default()
}

pub fn update_display_group_request() -> UpdateDisplayGroupRequestBuilder {
    UpdateDisplayGroupRequestBuilder::default()
}

pub fn unsubscribe_from_group_events_request() -> UnsubscribeFromGroupEventsRequestBuilder {
    UnsubscribeFromGroupEventsRequestBuilder::default()
}
