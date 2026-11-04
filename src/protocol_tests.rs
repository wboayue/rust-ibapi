use super::*;

#[test]
fn protocol_feature_new_constructs_runtime_value() {
    let feature = ProtocolFeature::new("custom feature", 200);
    assert_eq!(feature.name, "custom feature");
    assert_eq!(feature.min_version, 200);
}

#[test]
fn check_version_accepts_when_above_minimum() {
    let feature = Features::CONFIG;
    assert!(check_version(feature.min_version + 1, feature).is_ok());
}

#[test]
fn check_version_accepts_at_minimum_boundary() {
    let feature = Features::CONFIG;
    assert!(check_version(feature.min_version, feature).is_ok());
}

#[test]
fn check_version_rejects_below_minimum_with_canonical_display() {
    let feature = Features::CONFIG;
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
