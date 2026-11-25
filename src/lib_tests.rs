#[cfg(feature = "utoipa")]
use super::*;

// Guards the crate-root `utoipa` alias and the wire-form schemas: strings, hand-written (`Expiry`)
// or from `impl_wire_enum!` (open and closed), and integers from `impl_wire_code!`.
#[cfg(feature = "utoipa")]
#[test]
fn utoipa_schemas_resolve() {
    use utoipa::PartialSchema;

    let json = |schema| serde_json::to_value(schema).unwrap();
    let string = serde_json::json!({ "type": "string" });

    for (name, schema) in [
        ("OrderStatusKind", json(orders::OrderStatusKind::schema())),
        ("Expiry", json(contracts::Expiry::schema())),
        ("TimeInForce", json(orders::TimeInForce::schema())),
        ("OrderType", json(orders::OrderType::schema())),
        ("Action", json(orders::Action::schema())),
        ("WhatToShow", json(market_data::historical::WhatToShow::schema())),
    ] {
        assert_eq!(schema, string, "{name} schema");
    }

    for (name, schema) in [
        ("ShortSaleSlot", json(orders::ShortSaleSlot::schema())),
        ("OcaType", json(orders::OcaType::schema())),
        ("TickType", json(contracts::tick_types::TickType::schema())),
        ("FaDataType", json(accounts::FaDataType::schema())),
    ] {
        assert_eq!(schema["type"], "integer", "{name} schema: {schema}");
    }

    let contract = json(contracts::Contract::schema());
    assert!(contract["properties"]["symbol"].is_object(), "derived Contract schema: {contract}");
}
