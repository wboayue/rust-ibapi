//! Builders for fundamental-domain request and response messages.

use super::{RequestEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::contracts::Contract;
use crate::fundamental::FundamentalReportType;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{encode_contract, some_display, some_str};

#[derive(Clone, Debug)]
pub struct FundamentalDataRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub report_type: FundamentalReportType,
}

impl Default for FundamentalDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            report_type: FundamentalReportType::default(),
        }
    }
}

impl FundamentalDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, v: Contract) -> Self {
        self.contract = v;
        self
    }
    pub fn report_type(mut self, v: FundamentalReportType) -> Self {
        self.report_type = v;
        self
    }
}

impl RequestEncoder for FundamentalDataRequestBuilder {
    type Proto = proto::FundamentalsDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestFundamentalData;

    fn to_proto(&self) -> Self::Proto {
        proto::FundamentalsDataRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            report_type: some_display(Some(&self.report_type)),
            fundamentals_data_options: Default::default(),
        }
    }
}

pub fn fundamental_data_request() -> FundamentalDataRequestBuilder {
    FundamentalDataRequestBuilder::default()
}

// =============================================================================
// Response builders
// =============================================================================

#[derive(Clone, Debug)]
pub struct FundamentalDataResponse {
    pub request_id: i32,
    pub data: String,
}

impl Default for FundamentalDataResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            data: String::new(),
        }
    }
}

impl FundamentalDataResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn data(mut self, v: impl Into<String>) -> Self {
        self.data = v.into();
        self
    }
}

impl ResponseProtoEncoder for FundamentalDataResponse {
    type Proto = proto::FundamentalsData;

    fn to_proto(&self) -> Self::Proto {
        proto::FundamentalsData {
            req_id: Some(self.request_id),
            data: some_str(&self.data),
        }
    }
}

pub fn fundamental_data_response() -> FundamentalDataResponse {
    FundamentalDataResponse::default()
}
