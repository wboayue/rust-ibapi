# Issue #827: `OrderCondition` preserves an unmodeled condition type

Follow-up to #825 / `plans/orders-wire-enum-followups.md` §4. **Implemented** on branch `orders/condition-unknown-type`; open question resolved as recommended (`is_conjunction: bool`).

## Problem

- `decode_order_condition` (`src/proto/decoders.rs`) ends `_ => OrderCondition::Price(PriceCondition::default())`.
  An unmodeled type — including `2`, unassigned today, and `0` from an absent `type` —
  reads as a zeroed price condition. Worse than a lossy decode: re-placing that order
  (modify flow) sends a **different condition** to TWS.
- `From<i32> for OrderCondition` (`src/orders/mod.rs`) panics on unknown codes. Public,
  zero callers in `src/`, `examples/`, `docs/`, `integration/`.

## Wire facts (verified)

- Modeled types: 1 Price, 3 Time, 4 Margin, 5 Execution, 6 Volume, 7 PercentChange
  (`OrderCondition.cs` `OrderConditionType`).
- Reference client `EDecoderUtils.decodeConditions` (`:295-333`): `HasType ? Type : 0`, switch
  on known types, **silently drops** anything else. Not copying that: dropping loses the
  condition on a modify round-trip, same failure as today, just quieter.
- `proto::OrderCondition` is flat: 13 optional fields (`type`, `is_conjunction_connection`,
  `is_more`, `con_id`, `exchange`, `symbol`, `sec_type`, `percent`, `change_percent`,
  `price`, `trigger_method`, `time`, `volume`). So an unknown type can be preserved
  losslessly without knowing its semantics.

## Design

### `OrderCondition::Unknown(UnknownCondition)`

New struct in `src/orders/conditions.rs`, beside the six modeled ones:

```rust
/// A condition whose type this crate does not model, preserved as received so it
/// round-trips unchanged when the order is re-placed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UnknownCondition {
    pub condition_type: i32,
    pub is_conjunction: bool,
    pub is_more: Option<bool>,
    pub contract_id: Option<i32>,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub security_type: Option<String>,
    pub percent: Option<i32>,
    pub change_percent: Option<f64>,
    pub price: Option<f64>,
    pub trigger_method: Option<i32>,   // raw: meaning is type-specific
    pub time: Option<String>,
    pub volume: Option<i32>,
}
```

- Field names follow the modeled structs (`contract_id`, `security_type`), not proto names.
- `Option`s mirror proto presence so encode(decode(p)) == p — the whole point — except an
  absent conjunction flag, which returns as explicit AND (see open question).
- `is_conjunction: bool` (not `Option`) so `is_conjunction()` / `set_conjunction` stay
  uniform; decoded with the same `unwrap_or(true)` as the others.
- Why not `Unknown { condition_type, is_conjunction }`: lossy; re-placing would send a
  typed-but-empty condition. `Unknown(i32)` on the other enums is lossless because the code
  *is* the value; here the value is the whole message.
- Why not wrap `proto::OrderCondition`: `proto` is `pub(crate)`, no serde/utoipa.
- Enum stays exhaustive (`enum-typing.md`); derived serde gives `{"Unknown": {...}}`, fine —
  `OrderCondition` already serializes externally tagged.

### Accessors

- `condition_type()` → `Unknown(c) => c.condition_type`.
- `is_conjunction()` → `Unknown(c) => c.is_conjunction`.
- `set_conjunction` (`src/orders/builder/order_builder.rs`) gains the arm — reachable, since
  `.condition(impl Into<OrderCondition>)` accepts a hand-built `Unknown`.

### Decoder

`decode_order_condition` → `Result<OrderCondition, Error>`:

- `type` absent → `Error::Parse("missing OrderCondition type")`. `createConditionsProto`
  always sets it, so absent = malformed frame; matches #829's missing-submessage call and
  the rule's "missing → Error::Parse". (Confirm in `EClientUtils.cs` before coding.)
- Known code → as today.
- Any other code → `Unknown(UnknownCondition { .. })` copying every proto field.
- Call site (`decoders.rs:357`): `.map(decode_order_condition).collect::<Result<_, _>>()?`.

### Encoder

`encode_condition` (`src/proto/encoders.rs`) gains `Unknown(c)` arm writing each field back
verbatim (`r#type`/`is_conjunction_connection` already come from the accessors). No
`some_*_ne` filtering — presence is carried by the `Option`.

### Deletions

- `From<i32> for OrderCondition` — panicking, default-constructing, zero callers; builders
  in `conditions.rs` are the construction path. No `TryFrom` replacement (no consumer).
- `ToField for OrderCondition` / `ToField for Option<OrderCondition>` — text-wire leftovers,
  zero callers (`grep -rn to_field src | grep -i cond`). Verify with clippy/build after removal.

## Tests

`src/proto/decoders_tests.rs`:
- `decode_order_preserves_unknown_condition_type` — type `2` and `99` with populated fields →
  `Unknown` carrying each field.
- `decode_order_condition_missing_type_is_parse_error`.
- Probe: every code in `-8..=64` outside `{1,3,4,5,6,7}` decodes to `Unknown` with that
  `condition_type` — the variant-coverage guard, same range as `check_wire_code_round_trip`.
  Fails when a modeled type is added to the decoder without updating the probe's table.

`src/proto/encoders_tests.rs`:
- `encode_unknown_condition_round_trips` — `encode_condition(&decode_order_condition(&p)?) == p`
  for a fully populated and a sparsely populated `p`.

`src/orders/{sync,async}_tests.rs`:
- `order_update_stream_survives_unknown_condition` alongside
  `order_update_stream_survives_unknown_status` — OpenOrder frame with type-2 condition,
  stream yields it and continues. Use the existing testdata builders
  (`src/testdata/builders/orders.rs`); add a `conditions` setter only if none exists.

`src/orders/mod_tests.rs` (or sibling): `condition_type()` / `is_conjunction()` on `Unknown`;
serde round-trip of `Unknown` (coverage mop-up).

Coverage floor: new struct + arms fully exercised by the above.

## Docs

- `CHANGELOG.md` `[Unreleased]`:
  - Changed: `OrderCondition::Unknown(UnknownCondition)`; unmodeled type preserved and
    round-trips instead of reading as a zeroed price condition; exhaustive matches need the
    arm. Missing condition `type` → `Error::Parse`. See migration §16 (#827).
  - Removed: `From<i32> for OrderCondition`, `ToField` impls (#827).
- `docs/migration-4.0.md` §16: new arm; `OrderCondition::from(n)` → condition builders.
- `docs/rules/wire/enum-typing.md`: add `#827` to `precedents`; replace the #825 note's
  "left alone ... issue #827" tail with a #827 bullet — **message-shaped discriminator:
  the `Unknown` payload is the whole message, not the code** (lossless-round-trip criterion).
- `plans/orders-wire-enum-followups.md` §4: mark resolved by this PR.
- `docs/api-patterns.md`: no exhaustive matches; no change (re-grep before PR).

## Gate

Full pre-PR set from `CLAUDE.md`, plus (touches a proto decoder/encoder and public API):

```bash
cargo build -p ibapi-integration-sync --tests
cargo build -p ibapi-integration-async --tests
just rules-check
```

## Open question

`is_conjunction` default for `Unknown`: `unwrap_or(true)` matches the modeled types but means
an absent field re-encodes as `Some(true)` — the only non-verbatim field. Acceptable (C#
default is also AND), or make it `Option<bool>` and special-case the accessor. Recommend
accept; note it in the struct doc.
