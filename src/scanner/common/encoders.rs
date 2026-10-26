use crate::contracts::TagValue;
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{some_bool, some_i32_ne, tag_values_to_map};
use crate::Error;

use super::super::ScannerSubscription;

pub(in crate::scanner) fn encode_scanner_parameters() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(ScannerParametersRequest, OutgoingMessages::RequestScannerParameters)
}

pub(in crate::scanner) fn encode_scanner_subscription(
    request_id: i32,
    subscription: &ScannerSubscription,
    filter: &[TagValue],
) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::ScannerSubscriptionRequest {
        req_id: Some(request_id),
        scanner_subscription: Some(encode_subscription_params(subscription, filter)),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestScannerSubscription as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::scanner) fn encode_cancel_scanner_subscription(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelScannerSubscription, OutgoingMessages::CancelScannerSubscription)
}

// === Sub-message encoders (shared by the frame encoders above) ===

pub(crate) fn encode_subscription_params(subscription: &ScannerSubscription, filter: &[TagValue]) -> proto::ScannerSubscription {
    proto::ScannerSubscription {
        number_of_rows: some_i32_ne(subscription.number_of_rows, i32::MAX),
        instrument: subscription.instrument.clone(),
        location_code: subscription.location_code.clone(),
        scan_code: subscription.scan_code.clone(),
        above_price: subscription.above_price,
        below_price: subscription.below_price,
        above_volume: subscription.above_volume.map(|v| v as i64),
        market_cap_above: subscription.market_cap_above,
        market_cap_below: subscription.market_cap_below,
        moody_rating_above: subscription.moody_rating_above.clone(),
        moody_rating_below: subscription.moody_rating_below.clone(),
        sp_rating_above: subscription.sp_rating_above.clone(),
        sp_rating_below: subscription.sp_rating_below.clone(),
        maturity_date_above: subscription.maturity_date_above.clone(),
        maturity_date_below: subscription.maturity_date_below.clone(),
        coupon_rate_above: subscription.coupon_rate_above,
        coupon_rate_below: subscription.coupon_rate_below,
        exclude_convertible: some_bool(subscription.exclude_convertible),
        average_option_volume_above: subscription.average_option_volume_above.map(|v| v as i64),
        scanner_setting_pairs: subscription.scanner_setting_pairs.clone(),
        stock_type_filter: subscription.stock_type_filter.clone(),
        scanner_subscription_filter_options: tag_values_to_map(filter),
        scanner_subscription_options: Default::default(),
    }
}

// Encoder body assertions live in the migrated sync/async tests via
// `assert_request<B>(builder)`.
