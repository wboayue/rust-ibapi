use super::*;

#[test]
fn encode_subscription_params_omits_the_unset_row_count() {
    let unset = ScannerSubscription {
        number_of_rows: i32::MAX,
        ..Default::default()
    };
    assert_eq!(encode_subscription_params(&unset, &[]).number_of_rows, None);

    let ten = ScannerSubscription {
        number_of_rows: 10,
        ..Default::default()
    };
    assert_eq!(encode_subscription_params(&ten, &[]).number_of_rows, Some(10));
}

#[test]
fn encode_subscription_params_maps_filter_tags_to_options() {
    let filter = [TagValue {
        tag: "changePercAbove".to_string(),
        value: "5".to_string(),
    }];
    let proto = encode_subscription_params(&ScannerSubscription::default(), &filter);
    assert_eq!(
        proto.scanner_subscription_filter_options.get("changePercAbove").map(String::as_str),
        Some("5")
    );
    assert!(proto.scanner_subscription_options.is_empty());
}

#[test]
fn encode_subscription_params_widens_volume_fields() {
    let subscription = ScannerSubscription {
        above_volume: Some(1000),
        average_option_volume_above: Some(500),
        ..Default::default()
    };
    let proto = encode_subscription_params(&subscription, &[]);
    assert_eq!(proto.above_volume, Some(1000_i64));
    assert_eq!(proto.average_option_volume_above, Some(500_i64));
}
