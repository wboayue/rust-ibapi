use std::cell::Cell;

use time::macros::datetime;

use super::*;
use crate::contracts::OptionRight;
use crate::testdata::builders::orders::exercise_options_request;
use crate::testdata::builders::RequestEncoder;

fn contract() -> Contract {
    Contract::option("AAPL", "20251219", 150.0, OptionRight::Call)
}

/// Builder over a unit client: `encode` never touches the client.
fn builder(contract: &Contract) -> ExerciseOptionsBuilder<'_, ()> {
    ExerciseOptionsBuilder::new(&(), contract)
}

#[test]
fn setters_map_to_the_request() {
    let contract = contract();
    let base = exercise_options_request().order_id(13).contract(&contract);
    let mut with_time = base.clone();
    with_time.manual_order_time = Some("20251219 14:30:00 UTC".to_owned());

    let cases = [
        ("exercise", builder(&contract).exercise(2), base.clone().exercise_quantity(2)),
        (
            "lapse",
            builder(&contract).lapse(3),
            base.clone().exercise_action(ExerciseAction::Lapse).exercise_quantity(3),
        ),
        ("account", builder(&contract).exercise(1).account("DU123"), base.clone().account("DU123")),
        (
            "override",
            builder(&contract).exercise(1).override_natural_action(),
            base.clone().override_(true),
        ),
        (
            "manual_order_time",
            builder(&contract).exercise(1).manual_order_time(datetime!(2025-12-19 9:30 -5)),
            with_time,
        ),
        (
            "last action wins",
            builder(&contract).exercise(1).lapse(1),
            base.clone().exercise_action(ExerciseAction::Lapse),
        ),
    ];

    for (name, builder, expected) in cases {
        let (order_id, request) = builder.encode(|| 13).unwrap();
        assert_eq!(order_id, OrderId::from(13), "{name}");
        assert_eq!(request, expected.encode_request(), "{name}");
    }
}

#[test]
fn rejects_before_allocating_an_order_id() {
    let contract = contract();
    let cases = [
        (
            builder(&contract),
            Error::from(ValidationError::MissingRequiredField("exercise or lapse")),
        ),
        (builder(&contract).exercise(0), Error::from(ValidationError::InvalidQuantity(0.0))),
        (builder(&contract).lapse(-1), Error::from(ValidationError::InvalidQuantity(-1.0))),
    ];

    for (builder, expected) in cases {
        let allocated = Cell::new(false);
        let err = builder
            .encode(|| {
                allocated.set(true);
                13
            })
            .unwrap_err();
        assert_eq!(err.to_string(), expected.to_string());
        assert!(!allocated.get(), "{expected}");
    }
}

#[test]
fn rejects_an_order_id_in_the_request_range() {
    let contract = contract();
    let err = builder(&contract)
        .exercise(1)
        .encode(|| crate::client::ids::REQUEST_ID_FLOOR)
        .unwrap_err();
    assert!(matches!(err, Error::OrderIdInRequestRange { .. }), "{err:?}");
}
