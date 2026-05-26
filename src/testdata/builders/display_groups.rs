//! Builders for display-groups response messages.

use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::proto;
use crate::testdata::builders::ResponseProtoEncoder;

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
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }

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

pub fn display_group_updated() -> DisplayGroupUpdatedResponse {
    DisplayGroupUpdatedResponse::default()
}
