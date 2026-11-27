---
id: id-parameters
title: Ids go in typed, come out raw
cluster: style
status: active
triggers:
  - adding a public parameter that names an account, contract, model code or order
  - returning an id from a public method, or adding an id field to a public struct
  - adding an id newtype, or a conversion on one
symbols: [AccountId, ModelCode, ContractId, OrderId, BracketOrderIds, AttachedOrderIds]
related: [id-partition, param-budget, validation-vs-invalid-argument]
precedents: ["#1204", "#1250"]
---

**Parameters** take `impl Into<XId>`: `account: impl Into<AccountId>`,
`contract_id: impl Into<ContractId>`, `order_id: impl Into<OrderId>`. Never `&XId`, never the raw
type. An optional id goes on a builder setter (`.account(a)`), not `Option<impl Into<XId>>`,
which makes a bare `None` need a type annotation.

**Returns and struct fields** are raw: `i32` for order and contract ids, `String` for account ids
and model codes. `OrderBuilder::submit` returns `i32`, `BracketOrderIds` and `AttachedOrderIds`
hold `i32`, `Contract::contract_id` is `i32`, `managed_accounts()` returns `Vec<String>`. What
comes out feeds straight back into a typed parameter, and into the caller's own `HashMap<i32, _>`
or `let id: i32` without unwrapping. Crate internals may carry the newtype up to the public
boundary.

**Each newtype converts** `From<raw>`, `From<&raw>`, `From<XId> for raw`, and has `Display`.
`AccountId` and `ModelCode` also convert from `&str` and `&Self`. Homes: `accounts::{AccountId,
ModelCode}`, `contracts::ContractId`, `orders::OrderId`; all four are in the prelude.

Not yet conforming (#1250 moves them): `positions_multi` and `account_updates_multi` take
`Option<&AccountId>, Option<&ModelCode>`, `pnl`/`pnl_single` take `Option<&ModelCode>`,
`ContractBuilder::contract_id` takes `i32`, and the `ButterflyLegs`/`IronCondorLegs`/`CollarLegs`
fields are `ContractId`. Check: `grep -n "Option<&\(AccountId\|ModelCode\)>" src/accounts/{sync,async}.rs | grep -v ///`
(11 lines at #1250's first PR; zero when done).

Out of scope: ids with no newtype (`market_rule_id`, `article_id`, group ids). Don't add a
newtype just to apply this rule.

## Why

Typed parameters document the signature: `pnl_single(account, contract_id)` reads as which id
goes where. Raw outputs keep callers' code free of `.0` and `.value()`, and both directions meet
in the middle because every newtype converts from its raw value.

**This is not type safety.** `impl Into<ContractId>` accepts any `i32`, so passing an order id as
a contract id still compiles. Don't "fix" that by removing `From<i32>`: every value the crate
returns is raw, so the caller would have to wrap each one, and the mistake it would catch has not
been reported. Order-id range checking happens at use, not construction — see
[id partition](../wire/id-partition.md).

Before #1250 it held only in parts: `pnl*` took `&AccountId` while `option_chain` took `i32`
(#1204 aligned those), and `submit`/`submit_oca_orders`/`BracketOrderIds` returned `OrderId`,
so callers wrote `ids.parent.0`.

## Precedents

- #1204 — id parameters across `option_chain`, `wsh_event_data_by_contract`, `historical_news`,
  `pnl*`, `account_updates` and `OrderBuilder::account` moved to `impl Into<XId>`.
- #1250 — returned order ids raw (`submit`, `submit_oca_orders`, `BracketOrderIds`,
  `AttachedOrderIds`; `as_i32_vec` dropped as a duplicate of `as_vec`); `ContractId` moved to
  `contracts`; conversion set completed on all four newtypes. Integration crates held the only
  callers that unwrapped with `.0`.
