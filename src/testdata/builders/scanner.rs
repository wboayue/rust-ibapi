//! Builders for scanner-domain request messages.
//!
//! Response builders are intentionally absent: scanner responses use IB's
//! text wire format, and the existing inline literals in the migrated
//! sync/async tests already exercise the production decoders end-to-end.

use super::RequestEncoder;
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
