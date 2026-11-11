---
id: builder-enum-coverage
title: A fluent builder covers every variant of the enum it wraps
cluster: style
status: active
triggers:
  - adding a named method to a builder that sets an enum field
  - adding a variant to an enum a builder exposes
  - seeing an unreachable!() or panic!() arm in a caller matching on a builder-set enum
symbols: [OrderBuilder, Action, OrderType, unreachable]
related: [param-budget, domain-module-layout]
precedents: ["#549", "#822", "#832", "#833", "#1073", "#1076", "#1082"]
---

When a fluent builder exposes an enum through named methods (`.buy()` / `.sell()` for
`orders::Action`), every variant needs a way in. The field is private and the setter is the only
entry point, so a missing method is a variant callers cannot express. Adding a variant to such an
enum means adding its method in the same PR.

**The harder gap is an enum with no entry point at all**: a public enum on a public `Order` field
the builder cannot set. Before adding the setter, check the field reaches TWS. An untyped `i32`
standing in for the enum and a private field nothing writes are both symptoms, and the second can
mean the wire has no such field; see [wire enum typing](../wire/enum-typing.md) on
`AuctionStrategy`.

Open enums (`TimeInForce`, `Rule80A`, `OrderOpenClose`, `OrderType`) get no named method for
`Unknown(raw)`: the raw value comes from a decode, so the general setter is the way in, and a
`.unknown("Z")` method would invite invented wire strings.

## Auditing `Order`

Fourteen `Order` fields are enum-typed; nine have a same-named setter. Re-derive rather than trust
the count:

```bash
sed -n '/^pub struct Order {/,/^}/p' src/orders/mod.rs \
  | grep -oE '^    pub [a-z_0-9]+: (Option<)?(conditions::)?[A-Z][A-Za-z0-9]*>?,$' \
  | grep -vE ': (Option<)?(String|Vec)' | sed 's/^    pub //'
```

That prints fifteen rows; drop `soft_dollar_tier` (a struct). Look each field up in
`grep -oE 'pub fn [a-z_0-9]+' src/orders/builder/order_builder.rs`. Five misses are not gaps.
Three have setters named for the variants or a neighboring field: `action`
(`.buy()` / `.sell()` / `.sell_short()` / `.sell_long()`), `tif` (`.time_in_force()` and named
siblings), `oca_type` (`.oca_group(group, OcaType)`). Two have no builder setter at all:
`adjusted_order_type` is written by the free function `order_builder::attach_adjustable_stop`,
and `delta_neutral_order_type` by hand on a VOL order. The `Vec` filter also hides
`conditions: Vec<OrderCondition>`, reachable through `.condition()`.

`OrderType` is both the `order_type` field's type and what `.order_type(..)` takes. Cross-check
its variants against setters that write them (`Some(OrderType::X)` or
`self.order_type(OrderType::X)`):

```bash
sed -n '/^pub enum OrderType {/,/^}/p' src/orders/builder/types.rs \
  | grep -oE '^    [A-Z][A-Za-z]*,' | tr -d ' ,' \
  | while read v; do grep -v '^ *//' src/orders/builder/order_builder.rs \
      | grep -qE "(Some\(|order_type\()OrderType::$v\)" || echo "$v"; done
```

It prints nothing. `Unknown(String)` does not match the pattern and, as on the other open enums,
has no named method.

## Why

**The canary is an `unreachable!()` in someone else's code.** A caller matching on the enum must
write an arm for variants the builder cannot produce, and the honest arm is a panic. So the gap
shows up as `_ => unreachable!()` at a call site, not at the builder.

Nothing gates this. Exhaustiveness checking covers `match`, not "one method per variant", so the
check is a read: list variants, list setters, compare. Do it when you touch either side.

## Precedents

- #549 — `Action::SellShort` / `SellLong` were reachable only by hand-building the order;
  `.sell_short()` / `.sell_long()` closed the gap.
- #822 — `TimeInForce` gained `GoodTillCrossing` and `.good_till_crossing()` in the same PR.
- #832 — integer-coded enums: `OcaType` (bare `i32` on `oca_group`), `VolatilityType`,
  `TriggerMethod`, `OrderOrigin`, `ShortSaleSlot`, `ReferencePriceType` got enum-typed setters.
  `AuctionStrategy` had no proto field and was deleted: a missing setter can mean a dead field.
- #833 — the string-typed open enums: `.rule_80_a(Rule80A)`, `.open_close(OrderOpenClose)`.
- #1073 — `.order_type(MarketOnOpen | LimitOnOpen | AtAuction)` left TIF at `Day` and sent a
  plain day order; `order_type()` set the implied TIF and the named setters routed through it.
  Superseded in 6.0: the three variants, and `ComboLimit` / `ComboMarket`, shared a wire code
  with `Market` / `Limit` / `MarketToLimit` and were removed when `OrderType` became the
  `Order::order_type` field's type; `market_on_open()`, `limit_on_open()` and `at_auction()` set
  the type and TIF themselves (`docs/migration-6.0.md` §32).
- #1076 — `.relative_limit_combo()` / `.relative_market_combo()`, the `OrderType` gaps the
  cross-check recipe was added to surface. No offset argument, matching C# `OrderSamples`.
- #1082 — `.volatility_order(volatility, VolatilityType)` for `OrderType::Volatility`.
  `.volatility(..)` already set only the figure; changing it to set the type too would silently
  change existing callers' orders, so the new setter got a new name.
