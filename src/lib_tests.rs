#[cfg(feature = "utoipa")]
use super::*;

// Guards the crate-root `utoipa` alias and the wire-form schemas: strings, hand-written (`Expiry`)
// or from `impl_wire_enum!` / `impl_wire_string_serde!`, and integers from `impl_wire_code!`. A
// closed enum's schema lists its codes; an open one's accepts any value.
#[cfg(feature = "utoipa")]
#[test]
fn utoipa_schemas_resolve() {
    use serde_json::json;
    use utoipa::PartialSchema;

    let to_json = |schema| serde_json::to_value(schema).unwrap();

    for (name, schema) in [
        ("OrderStatusKind", to_json(orders::OrderStatusKind::schema())),
        ("Expiry", to_json(contracts::Expiry::schema())),
        ("TimeInForce", to_json(orders::TimeInForce::schema())),
        ("OrderType", to_json(orders::OrderType::schema())),
    ] {
        assert_eq!(schema, json!({ "type": "string" }), "open {name} schema");
    }

    // Written independently of the macros' variant lists.
    for (name, schema, codes) in [
        ("Action", to_json(orders::Action::schema()), json!(["BUY", "SELL", "SSHORT", "SLONG"])),
        ("LegAction", to_json(contracts::LegAction::schema()), json!(["BUY", "SELL", "SSHORT"])),
        ("OptionRight", to_json(contracts::OptionRight::schema()), json!(["C", "P"])),
        (
            "ExecutionFilterSide",
            to_json(orders::ExecutionFilterSide::schema()),
            json!(["BUY", "SELL"]),
        ),
        ("ExecutionSide", to_json(orders::ExecutionSide::schema()), json!(["BOT", "SLD"])),
        (
            "realtime WhatToShow",
            to_json(market_data::realtime::WhatToShow::schema()),
            json!(["TRADES", "MIDPOINT", "BID", "ASK", "AGGTRADES"]),
        ),
        (
            "historical WhatToShow",
            to_json(market_data::historical::WhatToShow::schema()),
            json!([
                "TRADES",
                "MIDPOINT",
                "BID",
                "ASK",
                "BID_ASK",
                "AGGTRADES",
                "HISTORICAL_VOLATILITY",
                "OPTION_IMPLIED_VOLATILITY",
                "FEE_RATE",
                "SCHEDULE",
                "ADJUSTED_LAST"
            ]),
        ),
        (
            "BarSize",
            to_json(market_data::historical::BarSize::schema()),
            json!([
                "1 secs", "5 secs", "10 secs", "15 secs", "30 secs", "1 min", "2 mins", "3 mins", "4 mins", "5 mins", "10 mins", "15 mins",
                "20 mins", "30 mins", "1 hour", "2 hours", "3 hours", "4 hours", "8 hours", "1 day", "1 week", "1 month"
            ]),
        ),
    ] {
        assert_eq!(schema, json!({ "type": "string", "enum": codes }), "closed {name} schema");
    }

    for (name, schema, codes) in [
        ("FaDataType", to_json(accounts::FaDataType::schema()), json!([1, 3])),
        ("ServerLogLevel", to_json(accounts::ServerLogLevel::schema()), json!([1, 2, 3, 4, 5])),
    ] {
        assert_eq!(schema["type"], "integer", "closed {name} schema: {schema}");
        assert_eq!(schema["enum"], codes, "closed {name} schema: {schema}");
    }

    for (name, schema) in [
        ("ShortSaleSlot", to_json(orders::ShortSaleSlot::schema())),
        ("OcaType", to_json(orders::OcaType::schema())),
        ("TickType", to_json(contracts::tick_types::TickType::schema())),
    ] {
        assert_eq!(schema["type"], "integer", "open {name} schema: {schema}");
        assert!(schema.get("enum").is_none(), "open {name} schema lists no codes: {schema}");
    }

    let contract = to_json(contracts::Contract::schema());
    assert!(contract["properties"]["symbol"].is_object(), "derived Contract schema: {contract}");
}
