# PR #825 review follow-ups

Review of [#825](https://github.com/wboayue/rust-ibapi/pull/825) (`orders: parse wire
enums through FromStr, preserving unknown values`) landed eight findings. #825 itself is
merged; this is the follow-up branch. Numbering below is the review's.

## Landing here

### 1. `decode_order`'s required-`action` invariant leaks one layer up

`decode_open_order_proto` / `decode_completed_order_proto`
(`src/orders/common/decoders/mod.rs`) do:

```rust
let order = p.order.as_ref().map(decode_order).transpose()?.unwrap_or_default();
```

So an `order` submessage that is **present but carries no `action`** is `Error::Parse`
(#825's change), while one that is **absent entirely** silently yields
`Order::default()` — `action == Buy`, the exact mishandling §14 says it removed. Same
for `contract` and `order_state`.

The reference client does neither: `EDecoder.cs:2652-2683` **drops the whole frame**,
returning before `eWrapper.openOrder(..)` if any of `Contract` / `Order` / `OrderState`
is null. It never synthesizes a default.

`decode_execution_data_proto` has the same shape, and so does its upstream counterpart
(`ExecutionDataEventProtoBuf`, `EDecoder.cs:2918`), so it is fixed in the same pass.

Fix: a missing submessage is `Error::Parse`, consistent with the missing-`action` call.
Handing the caller a phantom BUY order over an empty contract is worse than either
upstream's drop or a surfaced error.

### 2. `#[repr(i32)]` + explicit discriminants are decorative and a second source of truth

Once `From<T> for i32` is hand-written, `= 0` / `= 1` drive nothing. They are the only
reason `#[repr(i32)]` is needed (explicit discriminants on a data-carrying enum require
a primitive repr), and it is an ABI commitment in the public API buying nothing.

A missing match arm is a compile error, but `Foo = 5` paired with `Foo => 6` is not —
the discriminant and the match can silently disagree. Verified the removal compiles
clean under `--all-features`.

Sweep all eight, `Liquidity` included, so `enum-typing.md`'s shape description stays
one shape. `Liquidity` has no `From<Liquidity> for i32` at all, so its discriminants
were never read either.

### 3. `*_round_trips_every_wire_code` does not check "every"

Seven tests so named verify only the rows listed; adding `OcaType::Reserved = 7` breaks
none of them. #822 hit this and answered with `all_tifs_covers_every_variant`, and
`enum-typing.md` records it as the lesson.

Integer enums admit a cheaper guard than a per-enum `modeled_index` match: `From<i32>`
is total, so *every code outside the table must be `Unknown(code)`*. Give
`check_wire_code_round_trip` the `Unknown` constructor as a fn pointer and probe a code
range. A new modeled variant fails the probe.

### 5. `enum-typing.md`'s `AuctionStrategy` claim is false, and the field it names is dead

The node says `From<i32> for AuctionStrategy` "converts nothing but the `i32` a caller
hands `OrderBuilder`". `OrderBuilder::auction_strategy`
(`src/orders/builder/order_builder.rs:84`) is a **private field with no setter** — the
only writes are `None` at :147, and the `.into()` at :979 is unreachable.

Delete the dead field and correct the claim. The missing `.auction_strategy(..)` setter
is a `builder-enum-coverage` gap, tracked in issue #828 rather than adding public
API here.

**Resolved in #832, the other way round.** The setter was never addable: IBKR's
`source/proto/Order.proto` has no `auctionStrategy` field and
`EClientUtils.createOrderProto` never sets one, so at the protobuf floor
`Order::auction_strategy` was written by `auction_limit` and dropped by `encode_order`.
`AuctionStrategy`, the `Order` field and `auction_limit`'s fourth parameter are gone.
The six enums that *do* reach the wire got setters in the same PR.

Follow-ups left open by #832:

- With the strategy parameter gone, `auction_limit(action, quantity, price)` constructs
  exactly what `limit_order(action, quantity, price)` does. It survives only as a
  documented BOX-routing entry point. Third occurrence of a duplicate free constructor
  trips the `/simplify` rule of three — fold it into `limit_order` then.
- ~~#832 scoped its audit to the *integer-coded* enums, so two string-typed ones are still
  `builder-enum-coverage` gaps: `Order::rule_80_a` and `Order::open_close`.~~ **Closed by
  #833** with `pub fn rule_80_a(Rule80A)` / `pub fn open_close(OrderOpenClose)`. Both are
  real proto fields (`Order.proto` `rule80A = 29`, `openClose = 68`), checked first as
  #832 had to for `AuctionStrategy`. `OrderBuilder` now has a setter for every public enum
  on an `Order` field.

### 6. "All eight macros in `src/`" is nine

`grep -rn "macro_rules!" src --include=*.rs | wc -l` → 9. `impl_proto_payload!`
(`src/proto/payload.rs:32`, 25 impls) has no row. #825 re-derived the `impl_wire_enum!`
count on the line below but left the total.

### 7. `impl_wire_enum!` error-message asymmetry on empty input

The closed form reports `"unknown Action"` for empty input; the `fallback` form reports
`"empty $name"`. Only observable through a direct `"".parse()` — `parse_required`
intercepts empty first with `"missing {label}"` — but the two arms should agree.

## Deferred, with issues

### 4. `decode_order_condition` still collapses an unknown discriminator

`src/proto/decoders.rs` `_ => OrderCondition::Price(PriceCondition::default())` decodes
an unmodeled condition type as a zeroed price condition — §14's bug class, on §14's
path. `From<i32> for OrderCondition` (`src/orders/mod.rs`) also still carries a live
`panic!`, though nothing on the decode path reaches it. #825 deferred both explicitly
as "a separate change"; keeping that call. Tracked in issue #827.

**Resolved by #827** (`plans/order-condition-unknown-type.md`): `OrderCondition::Unknown(UnknownCondition)`
preserves every wire field and round-trips; absent `type` is `Error::Parse`; the panicking
`From<i32>` and the dead `ToField` impls are removed.

### 8. `Unknown(code)` aliasing a known code

`OcaType::Unknown(1) != OcaType::CancelWithBlock` under derived `PartialEq`, yet both
encode as `1`, so two `Order`s that serialize identically can compare unequal. Only
reachable by hand-constructing `Unknown` with a known code; decoders never produce it.
Inherited from the `Liquidity` / `TimeInForce` precedent. No action.
