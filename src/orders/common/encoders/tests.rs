use pretty_assertions::assert_eq;

use super::*;

#[test]
fn message_version_for() {
    assert_eq!(super::message_version_for(server_versions::NOT_HELD), 45);
    assert_eq!(super::message_version_for(server_versions::EXECUTION_DATA_CHAIN), 27);
}

#[test]
fn f64_max_to_zero() {
    assert_eq!(super::f64_max_to_zero(Some(f64::MAX)), Some(0.0));
    assert_eq!(super::f64_max_to_zero(Some(0.0)), Some(0.0));
    assert_eq!(super::f64_max_to_zero(Some(50.0)), Some(50.0));
}
