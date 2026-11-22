use super::*;
use crate::contracts::TagValue;

#[test]
fn encode_subscription_params_omits_the_unset_row_count() {
    let unset = ScannerSubscription {
        number_of_rows: i32::MAX,
        ..ScannerSubscription::new("STK", "TOP_PERC_GAIN")
    };
    assert_eq!(encode_subscription_params(&unset).number_of_rows, None);

    let ten = ScannerSubscription {
        number_of_rows: 10,
        ..ScannerSubscription::new("STK", "TOP_PERC_GAIN")
    };
    assert_eq!(encode_subscription_params(&ten).number_of_rows, Some(10));
}

#[test]
fn encode_subscription_params_maps_filter_tags_to_options() {
    let subscription = ScannerSubscription {
        filter_options: vec![TagValue {
            tag: "changePercAbove".to_string(),
            value: "5".to_string(),
        }],
        ..ScannerSubscription::new("STK", "TOP_PERC_GAIN")
    };
    let proto = encode_subscription_params(&subscription);
    assert_eq!(
        proto.scanner_subscription_filter_options.get("changePercAbove").map(String::as_str),
        Some("5")
    );
    assert!(proto.scanner_subscription_options.is_empty());
}

#[test]
fn encode_subscription_params_sends_volume_fields_as_i64() {
    let subscription = ScannerSubscription {
        above_volume: Some(i64::from(i32::MAX) + 1),
        average_option_volume_above: Some(500),
        ..ScannerSubscription::new("STK", "TOP_PERC_GAIN")
    };
    let proto = encode_subscription_params(&subscription);
    assert_eq!(proto.above_volume, Some(2_147_483_648));
    assert_eq!(proto.average_option_volume_above, Some(500_i64));
}

#[test]
fn encode_subscription_params_sends_the_required_fields_and_no_location() {
    let proto = encode_subscription_params(&ScannerSubscription::new("STK", "TOP_PERC_GAIN"));
    assert_eq!(proto.instrument.as_deref(), Some("STK"));
    assert_eq!(proto.scan_code.as_deref(), Some("TOP_PERC_GAIN"));
    assert_eq!(proto.location_code, None);
    assert_eq!(proto.number_of_rows, Some(-1));
    assert!(proto.scanner_subscription_filter_options.is_empty());
}

#[test]
fn encode_subscription_params_formats_maturity_dates_as_yyyymmdd() {
    let unset = encode_subscription_params(&ScannerSubscription::new("BOND", "NEAR_MATURITY_DATE"));
    assert_eq!((unset.maturity_date_above, unset.maturity_date_below), (None, None));

    let subscription = ScannerSubscription {
        maturity_date_above: Some(time::macros::date!(2030 - 01 - 01)),
        maturity_date_below: Some(time::macros::date!(2035 - 12 - 31)),
        ..ScannerSubscription::new("BOND", "NEAR_MATURITY_DATE")
    };
    let proto = encode_subscription_params(&subscription);
    assert_eq!(proto.maturity_date_above.as_deref(), Some("20300101"));
    assert_eq!(proto.maturity_date_below.as_deref(), Some("20351231"));
}
