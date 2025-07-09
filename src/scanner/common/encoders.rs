use crate::messages::{OutgoingMessages, RequestMessage};
use crate::orders::TagValue;
use crate::server_versions;
use crate::Error;

use super::super::ScannerSubscription;

pub(in crate::scanner) fn encode_scanner_parameters() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestScannerParameters);
    message.push_field(&VERSION);

    Ok(message)
}

pub(in crate::scanner) fn encode_scanner_subscription(
    request_id: i32,
    server_version: i32,
    subscription: &ScannerSubscription,
    filter: &Vec<TagValue>,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 4;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestScannerSubscription);
    if server_version < server_versions::SCANNER_GENERIC_OPTS {
        message.push_field(&VERSION);
    }
    message.push_field(&request_id);
    message.push_field(&subscription.number_of_rows);
    message.push_field(&subscription.instrument);
    message.push_field(&subscription.location_code);
    message.push_field(&subscription.scan_code);

    message.push_field(&subscription.above_price);
    message.push_field(&subscription.below_price);
    message.push_field(&subscription.above_volume);
    message.push_field(&subscription.market_cap_above);
    message.push_field(&subscription.market_cap_below);
    message.push_field(&subscription.moody_rating_above);
    message.push_field(&subscription.moody_rating_below);
    message.push_field(&subscription.sp_rating_above);
    message.push_field(&subscription.sp_rating_below);
    message.push_field(&subscription.maturity_date_above);
    message.push_field(&subscription.maturity_date_below);
    message.push_field(&subscription.coupon_rate_above);
    message.push_field(&subscription.coupon_rate_below);
    message.push_field(&subscription.exclude_convertible);
    message.push_field(&subscription.average_option_volume_above);
    message.push_field(&subscription.scanner_setting_pairs);
    message.push_field(&subscription.stock_type_filter);

    if server_version >= server_versions::SCANNER_GENERIC_OPTS {
        message.push_field(filter);
    }
    if server_version >= server_versions::LINKING {
        message.push_field(&""); // ignore subscription options
    }

    Ok(message)
}

pub(in crate::scanner) fn encode_cancel_scanner_subscription(request_id: i32) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::CancelScannerSubscription);
    message.push_field(&VERSION);
    message.push_field(&request_id);

    Ok(message)
}