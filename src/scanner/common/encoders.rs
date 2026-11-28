use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{some_bool, some_i32_ne, tag_values_to_map};
use crate::Error;

use super::super::ScannerSubscription;

pub(in crate::scanner) fn encode_scanner_parameters() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(ScannerParametersRequest, OutgoingMessages::RequestScannerParameters)
}

pub(in crate::scanner) fn encode_scanner_subscription(request_id: i32, subscription: &ScannerSubscription) -> Result<Vec<u8>, Error> {
    use crate::messages::encode_protobuf_message;
    use prost::Message;
    let request = crate::proto::ScannerSubscriptionRequest {
        req_id: Some(request_id),
        scanner_subscription: Some(encode_subscription_params(subscription)),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestScannerSubscription as i32,
        &request.encode_to_vec(),
    ))
}

pub(in crate::scanner) fn encode_cancel_scanner_subscription(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, CancelScannerSubscription, OutgoingMessages::CancelScannerSubscription)
}

// === Sub-message encoders (used by the frame encoders above) ===

pub(crate) fn encode_subscription_params(subscription: &ScannerSubscription) -> proto::ScannerSubscription {
    let yyyymmdd = time::macros::format_description!("[year][month][day]");
    let date = |date: Option<time::Date>| date.and_then(|d| d.format(yyyymmdd).ok());
    proto::ScannerSubscription {
        number_of_rows: some_i32_ne(subscription.number_of_rows, i32::MAX),
        instrument: Some(subscription.instrument.clone()),
        location_code: subscription.location_code.clone(),
        scan_code: Some(subscription.scan_code.clone()),
        above_price: subscription.above_price,
        below_price: subscription.below_price,
        above_volume: subscription.above_volume,
        market_cap_above: subscription.market_cap_above,
        market_cap_below: subscription.market_cap_below,
        moody_rating_above: subscription.moody_rating_above.clone(),
        moody_rating_below: subscription.moody_rating_below.clone(),
        sp_rating_above: subscription.sp_rating_above.clone(),
        sp_rating_below: subscription.sp_rating_below.clone(),
        maturity_date_above: date(subscription.maturity_date_above),
        maturity_date_below: date(subscription.maturity_date_below),
        coupon_rate_above: subscription.coupon_rate_above,
        coupon_rate_below: subscription.coupon_rate_below,
        exclude_convertible: some_bool(subscription.exclude_convertible),
        average_option_volume_above: subscription.average_option_volume_above,
        scanner_setting_pairs: subscription.scanner_setting_pairs.clone(),
        stock_type_filter: subscription.stock_type_filter.clone(),
        scanner_subscription_filter_options: tag_values_to_map(&subscription.filter_options),
        scanner_subscription_options: Default::default(),
    }
}

// The sync/async tests check the request envelope via `assert_request<B>(builder)`; their
// expected body comes from `encode_subscription_params` itself, so its field mapping is
// pinned in `encoders_tests.rs`.

#[cfg(test)]
#[path = "encoders_tests.rs"]
mod tests;
