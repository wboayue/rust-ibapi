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
precedents: ["#1204", "#1337"]
---

Id **parameters** take `impl Into<XId>`, never `&XId` or the raw type; an optional id is a
builder setter, not `Option<impl Into<XId>>`. Id **returns and public struct fields** are raw:
`i32` for order and contract ids, `String` for account ids and model codes. Every newtype
converts `From<raw>`, `From<&raw>` and `From<XId> for raw`, and implements `Display`
(`id_newtypes_convert_both_ways` in `src/prelude_tests.rs` pins the set).

## Detail

`account: impl Into<AccountId>`, `contract_id: impl Into<ContractId>`,
`order_id: impl Into<OrderId>`. `Option<impl Into<XId>>` makes a bare `None` need a type
annotation, so an optional id goes on a builder (`.account(a)`).

`OrderBuilder::submit` returns `i32`, `BracketOrderIds` and `AttachedOrderIds` hold `i32`,
`Contract::contract_id` is `i32`, `managed_accounts()` returns `Vec<String>`. Crate internals may
carry the newtype up to the public boundary.

`AccountId` and `ModelCode` also convert from `&str` and `&Self`. Homes:
`accounts::{AccountId, ModelCode}`, `contracts::ContractId`, `orders::OrderId`; all four are in
the prelude.

Out of scope:
- Ids with no newtype (`market_rule_id`, `article_id`, group ids). Don't add a newtype just to
  apply this rule.
- A lookup key over data the crate returned: `AccountSummarySnapshot::get(account: &str, ..)`
  searches raw `String` fields.
- An account that is not the caller's: `ExerciseOptionsBuilder::customer_account` names a
  customer's account, which `AccountId` does not model.

## Why

Typed parameters document the signature: the type in rustdoc and IDE hints names which id goes
where. Raw outputs keep callers' code free of `.0` and `.value()`, and what comes out feeds
straight back into a typed parameter, a `HashMap<i32, _>` or a `let id: i32`.

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
- #1337 — returned order ids raw (`submit`, `submit_oca_orders`, `BracketOrderIds`,
  `AttachedOrderIds`; `as_i32_vec` dropped as a duplicate of `as_vec`); `ContractId` moved to
  `contracts`; conversion set completed on all four newtypes and pinned by a test. Integration
  crates held the only callers that unwrapped with `.0`.
