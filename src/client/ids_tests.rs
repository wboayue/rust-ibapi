use super::*;

#[test]
fn classify_by_range() {
    let floor = REQUEST_ID_FLOOR;
    let cases = [
        (-1, None),
        (-5, Some(WireId::Order(OrderId(-5)))),
        (0, Some(WireId::Order(OrderId(0)))),
        (floor - 1, Some(WireId::Order(OrderId(floor - 1)))),
        (floor, Some(WireId::Request(RequestId(floor)))),
        (REQUEST_ID_CEILING, Some(WireId::Request(RequestId(REQUEST_ID_CEILING)))),
        (i32::MAX, Some(WireId::Request(RequestId(i32::MAX)))),
    ];
    for (id, expected) in cases {
        assert_eq!(WireId::classify(id), expected, "id {id}");
    }
}

#[test]
fn request_id_from_raw_rejects_below_floor() {
    assert_eq!(RequestId::from_raw(REQUEST_ID_FLOOR - 1), None);
    assert_eq!(RequestId::from_raw(REQUEST_ID_FLOOR).map(RequestId::raw), Some(REQUEST_ID_FLOOR));
}

#[test]
fn order_id_checked_rejects_request_range() {
    assert_eq!(OrderId::from(REQUEST_ID_FLOOR - 1).checked().unwrap().value(), REQUEST_ID_FLOOR - 1);
    assert_eq!(OrderId::from(-5).checked().unwrap().value(), -5);

    let err = OrderId::from(REQUEST_ID_FLOOR).checked().unwrap_err();
    assert!(
        matches!(err, Error::OrderIdInRequestRange { order_id } if order_id == REQUEST_ID_FLOOR),
        "{err:?}"
    );
}

#[test]
fn display_is_the_raw_value() {
    assert_eq!(OrderId::from(42).to_string(), "42");
    assert_eq!(RequestId(REQUEST_ID_FLOOR).to_string(), REQUEST_ID_FLOOR.to_string());
}
