//! Builders for WSH-domain request messages.
//!
//! Response builders are absent: WSH responses are JSON payloads on the
//! existing text wire (see `wsh::common::test_data::build_response`).

use super::RequestEncoder;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::wsh::AutoFill;
use time::Date;

const DATE_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!("[year][month][day]");

single_req_id_request_builder!(WshMetadataRequestBuilder, WshMetaDataRequest, OutgoingMessages::RequestWshMetaData);

single_req_id_request_builder!(CancelWshMetadataRequestBuilder, CancelWshMetaData, OutgoingMessages::CancelWshMetaData);

single_req_id_request_builder!(CancelWshEventDataRequestBuilder, CancelWshEventData, OutgoingMessages::CancelWshEventData);

#[derive(Clone, Debug)]
pub struct WshEventDataRequestBuilder {
    pub request_id: i32,
    pub contract_id: Option<i32>,
    pub filter: Option<String>,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub limit: Option<i32>,
    pub auto_fill: Option<AutoFill>,
}

impl Default for WshEventDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract_id: None,
            filter: None,
            start_date: None,
            end_date: None,
            limit: None,
            auto_fill: None,
        }
    }
}

impl WshEventDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract_id(mut self, v: Option<i32>) -> Self {
        self.contract_id = v;
        self
    }
    pub fn filter(mut self, v: Option<&str>) -> Self {
        self.filter = v.map(|s| s.to_string());
        self
    }
    pub fn start_date(mut self, v: Option<Date>) -> Self {
        self.start_date = v;
        self
    }
    pub fn end_date(mut self, v: Option<Date>) -> Self {
        self.end_date = v;
        self
    }
    pub fn limit(mut self, v: Option<i32>) -> Self {
        self.limit = v;
        self
    }
    pub fn auto_fill(mut self, v: Option<AutoFill>) -> Self {
        self.auto_fill = v;
        self
    }
}

impl RequestEncoder for WshEventDataRequestBuilder {
    type Proto = proto::WshEventDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestWshEventData;

    fn to_proto(&self) -> Self::Proto {
        proto::WshEventDataRequest {
            req_id: Some(self.request_id),
            con_id: self.contract_id,
            filter: self.filter.clone(),
            fill_watchlist: self.auto_fill.as_ref().map(|af| af.watchlist),
            fill_portfolio: self.auto_fill.as_ref().map(|af| af.portfolio),
            fill_competitors: self.auto_fill.as_ref().map(|af| af.competitors),
            start_date: self.start_date.and_then(|d| d.format(DATE_FORMAT).ok()),
            end_date: self.end_date.and_then(|d| d.format(DATE_FORMAT).ok()),
            total_limit: self.limit,
        }
    }
}

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn wsh_metadata_request() -> WshMetadataRequestBuilder {
    WshMetadataRequestBuilder::default()
}

pub fn cancel_wsh_metadata_request() -> CancelWshMetadataRequestBuilder {
    CancelWshMetadataRequestBuilder::default()
}

pub fn wsh_event_data_request() -> WshEventDataRequestBuilder {
    WshEventDataRequestBuilder::default()
}

pub fn cancel_wsh_event_data_request() -> CancelWshEventDataRequestBuilder {
    CancelWshEventDataRequestBuilder::default()
}
