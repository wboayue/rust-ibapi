//! Builders for scanner-domain request and response messages.

use super::{RequestEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::proto;
use crate::scanner::ScannerSubscription;

empty_request_builder!(
    ScannerParametersRequestBuilder,
    ScannerParametersRequest,
    OutgoingMessages::RequestScannerParameters
);

single_req_id_request_builder!(
    CancelScannerSubscriptionRequestBuilder,
    CancelScannerSubscription,
    OutgoingMessages::CancelScannerSubscription
);

#[derive(Clone, Debug)]
pub struct ScannerSubscriptionRequestBuilder {
    pub request_id: i32,
    pub subscription: ScannerSubscription,
    pub filter: Vec<TagValue>,
}

impl Default for ScannerSubscriptionRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            subscription: ScannerSubscription::default(),
            filter: Vec::new(),
        }
    }
}

impl ScannerSubscriptionRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn subscription(mut self, v: &ScannerSubscription) -> Self {
        self.subscription = v.clone();
        self
    }
    pub fn filter(mut self, v: &[TagValue]) -> Self {
        self.filter = v.to_vec();
        self
    }
}

impl RequestEncoder for ScannerSubscriptionRequestBuilder {
    type Proto = proto::ScannerSubscriptionRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestScannerSubscription;

    fn to_proto(&self) -> Self::Proto {
        proto::ScannerSubscriptionRequest {
            req_id: Some(self.request_id),
            scanner_subscription: Some(proto::encoders::encode_scanner_subscription(&self.subscription, &self.filter)),
        }
    }
}

// =============================================================================
// Response builders
// =============================================================================

/// Builder for `ScannerParameters` (msg 19) responses.
#[derive(Clone, Debug)]
pub struct ScannerParametersResponse {
    pub xml: String,
}

impl Default for ScannerParametersResponse {
    fn default() -> Self {
        Self {
            xml: r#"<?xml version="1.0" encoding="UTF-8"?>
<ScanParameterResponse>
<InstrumentList>...</InstrumentList>
</ScanParameterResponse>"#
                .to_string(),
        }
    }
}

impl ScannerParametersResponse {
    pub fn xml(mut self, v: impl Into<String>) -> Self {
        self.xml = v.into();
        self
    }
}

impl ResponseProtoEncoder for ScannerParametersResponse {
    type Proto = proto::ScannerParameters;

    fn to_proto(&self) -> Self::Proto {
        proto::ScannerParameters { xml: Some(self.xml.clone()) }
    }
}

/// One row of a `ScannerData` response.
#[derive(Clone, Debug)]
pub struct ScannerDataRow {
    pub rank: i32,
    pub contract_id: i32,
    pub symbol: String,
    pub security_type: String,
    pub exchange: String,
    pub currency: String,
    pub local_symbol: String,
    pub trading_class: String,
    pub market_name: String,
    pub combo_key: String,
}

impl ScannerDataRow {
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn market_name(mut self, v: impl Into<String>) -> Self {
        self.market_name = v.into();
        self
    }
}

/// Builder for `ScannerData` (msg 20) responses.
#[derive(Clone, Debug)]
pub struct ScannerDataResponse {
    pub request_id: i32,
    pub rows: Vec<ScannerDataRow>,
}

impl Default for ScannerDataResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            rows: Vec::new(),
        }
    }
}

impl ScannerDataResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn row(mut self, row: ScannerDataRow) -> Self {
        self.rows.push(row);
        self
    }
    pub fn rows(mut self, rows: Vec<ScannerDataRow>) -> Self {
        self.rows = rows;
        self
    }
}

impl ResponseProtoEncoder for ScannerDataResponse {
    type Proto = proto::ScannerData;

    fn to_proto(&self) -> Self::Proto {
        proto::ScannerData {
            req_id: Some(self.request_id),
            scanner_data_element: self
                .rows
                .iter()
                .map(|row| proto::ScannerDataElement {
                    rank: Some(row.rank),
                    contract: Some(proto::Contract {
                        con_id: Some(row.contract_id),
                        symbol: Some(row.symbol.clone()),
                        sec_type: Some(row.security_type.clone()),
                        exchange: Some(row.exchange.clone()),
                        currency: Some(row.currency.clone()),
                        local_symbol: Some(row.local_symbol.clone()),
                        trading_class: Some(row.trading_class.clone()),
                        ..Default::default()
                    }),
                    market_name: Some(row.market_name.clone()),
                    distance: None,
                    benchmark: None,
                    projection: None,
                    combo_key: Some(row.combo_key.clone()),
                })
                .collect(),
        }
    }
}

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn scanner_parameters_request() -> ScannerParametersRequestBuilder {
    ScannerParametersRequestBuilder
}

pub fn scanner_subscription_request() -> ScannerSubscriptionRequestBuilder {
    ScannerSubscriptionRequestBuilder::default()
}

pub fn cancel_scanner_subscription_request() -> CancelScannerSubscriptionRequestBuilder {
    CancelScannerSubscriptionRequestBuilder::default()
}

pub fn scanner_parameters() -> ScannerParametersResponse {
    ScannerParametersResponse::default()
}

pub fn scanner_data() -> ScannerDataResponse {
    ScannerDataResponse::default()
}

pub fn scanner_data_row(rank: i32, contract_id: i32, symbol: impl Into<String>) -> ScannerDataRow {
    ScannerDataRow {
        rank,
        contract_id,
        symbol: symbol.into(),
        security_type: "STK".to_string(),
        exchange: "SMART".to_string(),
        currency: "USD".to_string(),
        local_symbol: String::new(),
        trading_class: "NMS".to_string(),
        market_name: "NMS".to_string(),
        combo_key: String::new(),
    }
}
