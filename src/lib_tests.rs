#[cfg(feature = "utoipa")]
use super::*;

// Guards the crate-root `utoipa` alias and the hand-written wire-string schemas.
#[cfg(feature = "utoipa")]
#[test]
fn utoipa_schemas_resolve() {
    use utoipa::PartialSchema;

    let json = |schema| serde_json::to_value(schema).unwrap();
    let string = serde_json::json!({ "type": "string" });

    for (name, schema) in [
        ("OrderStatusKind", json(orders::OrderStatusKind::schema())),
        ("TimeInForce", json(orders::TimeInForce::schema())),
    ] {
        assert_eq!(schema, string, "{name} schema");
    }

    let contract = json(contracts::Contract::schema());
    assert!(contract["properties"]["symbol"].is_object(), "derived Contract schema: {contract}");
}
