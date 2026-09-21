---
id: builder-enum-coverage
title: A fluent builder covers every variant of the enum it wraps
cluster: style
status: active
triggers:
  - adding a named method to a builder that sets an enum field
  - adding a variant to an enum a builder exposes
  - seeing an unreachable!() or panic!() arm in a caller matching on a builder-set enum
symbols: [OrderBuilder, Action, unreachable]
related: [param-budget, domain-module-layout]
precedents: ["#549", "#822", "#832"]
memory: [feedback_builder_enum_coverage_audit]
---

When a fluent builder exposes an enum through named methods — `.buy()` / `.sell()` for
`orders::Action`, and the like — every variant of that enum needs a method. A
builder that covers four of six variants is a builder that cannot express the other two, and the
caller has no fallback: the field is private and the setter is the only way in.

Adding a variant to such an enum means adding the method in the same PR.

**The harder gap is an enum with no entry point at all.** A public enum on a public `Order`
field that the builder cannot set is unreachable through the surface the builder exists to
replace. Before adding the setter, check the field reaches TWS: an untyped `i32` parameter
standing in for the enum (`oca_group(group, 1)`) and a private field nothing writes are both
symptoms, and the second can mean the wire has no such field — see
[wire enum typing](../wire/enum-typing.md) on `AuctionStrategy`.

## Why

**The canary is an `unreachable!()` in someone else's code.** A caller matching on the enum has
to write an arm for the variants the builder cannot produce, and the honest thing to write there
is a panic — so the gap shows up as `_ => unreachable!()` at a call site rather than as anything
missing at the builder. #549 found `Action::SellShort` and `Action::SellLong` reachable only by
constructing the order struct by hand, which is the surface the builder exists to replace.

Nothing gates this. Rust's exhaustiveness checking covers `match`, not "is there a method per
variant", so the check is a read: list the variants, list the setters, compare. Do it when you
touch either side.

## Precedents

- #549 — `Action::SellShort` / `Action::SellLong` were reachable only by hand-building the
  order struct; `.sell_short()` / `.sell_long()` closed the gap.
- #832 — the audit #828 asked for. Six of the seven integer-coded order enums had no entry
  point: `OcaType` was reachable only as a bare `i32` on `oca_group`, `VolatilityType` through
  a private field with no setter, and `TriggerMethod` / `OrderOrigin` / `ShortSaleSlot` /
  `ReferencePriceType` not at all. All six got a setter taking the enum. The seventh,
  `AuctionStrategy`, had no proto field behind it and was deleted instead — the
  counter-example: a missing setter is sometimes the honest signal that the field is dead.
- #822 — `TimeInForce` gained `GoodTillCrossing` and `.good_till_crossing()` in the same PR,
  as the directive says. Its open-enum `Unknown(raw)` arm is the one variant with no named
  method, deliberately: the raw value comes from a decode, so the general
  `.time_in_force(..)` setter is the only sensible way in and a `.unknown("GTZ")` method
  would read as an invitation to invent wire strings.
