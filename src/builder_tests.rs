use super::*;

#[test]
fn missing_marker_is_copyable_and_debug() {
    let m = Missing;
    let copy = m;
    let _ = copy;
    assert_eq!(format!("{:?}", Missing), "Missing");
}
