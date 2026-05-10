use super::*;

#[test]
fn protocol_feature_new_constructs_runtime_value() {
    let feature = ProtocolFeature::new("custom feature", 200);
    assert_eq!(feature.name, "custom feature");
    assert_eq!(feature.min_version, 200);
}

#[test]
fn check_version_accepts_when_above_minimum() {
    let result = check_version(150, Features::POSITIONS);
    assert!(result.is_ok());
}

#[test]
fn check_version_accepts_at_minimum_boundary() {
    let feature = Features::TICK_BY_TICK;
    assert!(check_version(feature.min_version, feature).is_ok());
}

#[test]
fn check_version_rejects_below_minimum_with_canonical_display() {
    let feature = Features::TICK_BY_TICK;
    let actual_version = feature.min_version - 1;
    let err = check_version(actual_version, feature).unwrap_err();
    match &err {
        Error::ServerVersion(required, actual, name) => {
            assert_eq!(*required, feature.min_version);
            assert_eq!(*actual, actual_version);
            assert_eq!(name, feature.name);
        }
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
    assert_eq!(
        err.to_string(),
        format!(
            "server version {} required, got {}: {}",
            feature.min_version, actual_version, feature.name
        ),
    );
}

#[test]
fn is_supported_returns_true_above_or_at_minimum() {
    assert!(is_supported(150, Features::POSITIONS));
    let feature = Features::TICK_BY_TICK;
    assert!(is_supported(feature.min_version, feature));
}

#[test]
fn is_supported_returns_false_below_minimum() {
    assert!(!is_supported(50, Features::TICK_BY_TICK));
    let feature = Features::TICK_BY_TICK;
    assert!(!is_supported(feature.min_version - 1, feature));
}

#[test]
fn include_if_supported_invokes_closure_when_supported() {
    let mut called = false;
    include_if_supported(150, Features::POSITIONS, || {
        called = true;
    });
    assert!(called);
}

#[test]
fn include_if_supported_skips_closure_when_unsupported() {
    let mut called = false;
    include_if_supported(50, Features::TICK_BY_TICK, || {
        called = true;
    });
    assert!(!called);
}

#[test]
fn include_if_supported_invokes_closure_at_minimum_boundary() {
    let feature = Features::CASH_QTY;
    let mut called = false;
    include_if_supported(feature.min_version, feature, || {
        called = true;
    });
    assert!(called);
}
