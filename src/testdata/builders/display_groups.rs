//! Builders for display-groups response messages.

use crate::proto;
use crate::testdata::builders::ResponseProtoEncoder;

#[derive(Clone, Debug, Default)]
pub struct DisplayGroupUpdatedResponse {
    pub request_id: Option<i32>,
    pub contract_info: String,
}

impl DisplayGroupUpdatedResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = Some(v);
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
            req_id: self.request_id,
            contract_info: Some(self.contract_info.clone()),
        }
    }
}

pub fn display_group_updated() -> DisplayGroupUpdatedResponse {
    DisplayGroupUpdatedResponse::default()
}
