#[cfg(feature = "utoipa")]
use super::*;

// Guards the crate-root `utoipa` alias and the wire-string schemas, hand-written (`Expiry`) or
// from `impl_wire_enum!(.., serde)`, and the wire-code schemas from `impl_wire_code!(.., serde { .. })`.
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
    ] {
        assert_eq!(schema, string, "{name} schema");
    }

    // From `impl_wire_code!(.., serde { .. })`.
    let short_sale_slot = json(orders::ShortSaleSlot::schema());
    assert_eq!(short_sale_slot["type"], "integer", "ShortSaleSlot schema: {short_sale_slot}");

    let contract = json(contracts::Contract::schema());
    assert!(contract["properties"]["symbol"].is_object(), "derived Contract schema: {contract}");
}
