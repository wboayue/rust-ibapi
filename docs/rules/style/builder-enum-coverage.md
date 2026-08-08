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
precedents: ["#549"]
memory: [feedback_builder_enum_coverage_audit]
---

When a fluent builder exposes an enum through named methods — `.buy()` / `.sell()` for
`orders::Action`, and the like — every variant of that enum needs a method. A
builder that covers four of six variants is a builder that cannot express the other two, and the
caller has no fallback: the field is private and the setter is the only way in.

Adding a variant to such an enum means adding the method in the same PR.

## Why

**The canary is an `unreachable!()` in someone else's code.** A caller matching on the enum has
to write an arm for the variants the builder cannot produce, and the honest thing to write there
is a panic — so the gap shows up as `_ => unreachable!()` at a call site rather than as anything
missing at the builder. #549 found `Action::SellShort` and `Action::SellLong` reachable only by
constructing the order struct by hand, which is the surface the builder exists to replace.

Nothing gates this. Rust's exhaustiveness checking covers `match`, not "is there a method per
variant", so the check is a read: list the variants, list the setters, compare. Do it when you
touch either side.
