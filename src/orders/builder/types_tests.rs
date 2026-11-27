use super::*;

#[test]
fn bracket_order_ids() {
    let ids = BracketOrderIds::new(100, 101, 102);
    assert_eq!((ids.parent, ids.take_profit, ids.stop_loss), (100, 101, 102));
    assert_eq!(ids.as_vec(), vec![100, 101, 102]);

    let ids2 = BracketOrderIds::try_from(vec![200, 201, 202]).unwrap();
    assert_eq!(ids2.parent, 200);

    let ids3 = BracketOrderIds::from([300, 301, 302]);
    assert_eq!(ids3.parent, 300);
}

#[test]
fn quantity_validation() {
    assert!(Quantity::new(100.0).is_ok());
    assert!(Quantity::new(0.0).is_err());
    assert!(Quantity::new(-10.0).is_err());
    assert!(Quantity::new(f64::NAN).is_err());
    assert!(Quantity::new(f64::INFINITY).is_err());
}

#[test]
fn price_validation() {
    assert!(Price::new(50.0).is_ok());
    assert!(Price::new(0.0).is_ok());
    assert!(Price::new(-10.0).is_ok());
    assert!(Price::new(f64::NAN).is_err());
    assert!(Price::new(f64::INFINITY).is_err());
}

#[test]
fn bracket_order_ids_wrong_length() {
    for ids in [vec![], vec![100, 101], vec![100, 101, 102, 103]] {
        let expected = Err(ValidationError::InvalidBracketOrder(format!("expected 3 order ids, got {}", ids.len())));
        assert_eq!(BracketOrderIds::try_from(ids), expected);
    }
}

#[test]
fn order_ids_serialize_as_integers() {
    let bracket = serde_json::to_value(BracketOrderIds::new(1, 2, 3)).unwrap();
    assert_eq!(bracket, serde_json::json!({"parent": 1, "take_profit": 2, "stop_loss": 3}));

    let attached = AttachedOrderIds {
        parent: 1,
        stop_loss: Some(2),
        profit_taker: None,
    };
    let json = serde_json::to_value(attached).unwrap();
    assert_eq!(json, serde_json::json!({"parent": 1, "stop_loss": 2, "profit_taker": null}));
}
