#[allow(dead_code)]
pub fn assert_send_and_sync<T: Send + Sync>() {}

#[test]
fn encodes_max_f64_as_tws_unset_double() {
    use crate::ToField;

    assert_eq!(f64::MAX.to_field(), "1.7976931348623157E308");
}
