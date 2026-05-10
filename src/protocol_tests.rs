use super::*;

#[test]
fn protocol_feature_new_constructs_runtime_value() {
    // ProtocolFeature::new is a `const fn` consumed by the Features:: constants;
    // the runtime call exercises the function body directly.
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
    // server_version == min_version must be supported (>= comparison).
    let feature = Features::TICK_BY_TICK;
    assert!(check_version(feature.min_version, feature).is_ok());
}

#[test]
fn check_version_rejects_just_below_minimum() {
    let feature = Features::TICK_BY_TICK;
    let result = check_version(feature.min_version - 1, feature);
    let err = result.unwrap_err();
    match err {
        Error::ServerVersion(required, actual, name) => {
            assert_eq!(required, feature.min_version);
            assert_eq!(actual, feature.min_version - 1);
            assert_eq!(name, feature.name);
        }
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
}

#[test]
fn check_version_unsupported_renders_canonical_display() {
    let result = check_version(50, Features::TICK_BY_TICK);
    let err = result.unwrap_err();
    match &err {
        Error::ServerVersion(required, actual, feature) => {
            assert_eq!(*required, 137);
            assert_eq!(*actual, 50);
            assert_eq!(feature, "tick-by-tick data");
        }
        other => panic!("expected ServerVersion error, got {other:?}"),
    }

    // Display formatting must match the variant convention (errors.rs).
    assert_eq!(err.to_string(), "server version 137 required, got 50: tick-by-tick data");
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
