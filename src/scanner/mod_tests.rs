use super::*;
use time::macros::date;

#[test]
fn scanner_subscription_deserializes_without_filter_options() {
    let subscription = ScannerSubscription::new("STK", "TOP_PERC_GAIN");
    let mut json = serde_json::to_value(&subscription).unwrap();
    assert!(json.as_object_mut().unwrap().remove("filter_options").is_some());

    let decoded: ScannerSubscription = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, subscription);
}

#[test]
fn scanner_subscription_serializes_a_maturity_date_as_year_and_ordinal() {
    let subscription = ScannerSubscription {
        maturity_date_above: Some(date!(2030 - 01 - 01)),
        ..ScannerSubscription::new("BOND", "NEAR_MATURITY_DATE")
    };
    let json = serde_json::to_value(&subscription).unwrap();
    assert_eq!(json["maturity_date_above"], serde_json::json!([2030, 1]));
    assert_eq!(json["instrument"], "BOND");

    let decoded: ScannerSubscription = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, subscription);
}
