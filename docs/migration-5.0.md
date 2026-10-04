# Migration Guide: 4.x to 5.0

This guide covers the breaking changes in `ibapi` 5.0. For 3.x → 4.x, see [`migration-4.0.md`](migration-4.0.md).

## Breaking changes

### 1. `OrderCondition` gains `Unknown(UnknownCondition)`

The last order wire enum that coerced an unrecognized value. `decode_order_condition` read any condition type it did not model — including `2`, which IB leaves unassigned between `Price = 1` and `Time = 3` — as `OrderCondition::Price(PriceCondition::default())`: contract id `0`, empty exchange, price `0.0`, indistinguishable from a real price condition. Re-placing that order sent TWS a different condition from the one it held.

- **An unmodeled type decodes as `OrderCondition::Unknown(UnknownCondition)`**, carrying the type code and every field of the wire condition as received (`Option`s mirror presence). `encode_order` writes it back unchanged, so an order read from TWS and placed again keeps its condition. `condition_type()` returns the raw code and `is_conjunction()` the flag; `OrderBuilder`'s `.and_condition(..)` / `.or_condition(..)` set it like any other. The reference client drops such a condition outright, which would lose it on the same round-trip.
- **A condition with no `type` fails to decode with `Error::Parse`.** The reference client always sets it; as in [4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values), the subscription that received the frame yields the error and ends.
- **`From<i32> for OrderCondition` is removed.** It built a default-valued condition from a type code and panicked on any other; use the condition builders (`PriceCondition::builder(..)`, `orders::builder::price(..)`, …) or construct the variant directly.
- **`ToField for OrderCondition` and `ToField for Option<OrderCondition>` are removed** — leftovers from the text wire format with no remaining caller.

```rust,ignore
// 4.2
let condition = OrderCondition::from(1);                 // zeroed PriceCondition; panicked on 2

// 5.0
let condition = OrderCondition::Price(PriceCondition::builder(265598, "SMART").greater_than(150.0).build());
match condition {
    OrderCondition::Price(_) => { /* ...and the other five modeled types */ }
    OrderCondition::Unknown(c) => eprintln!("unmodeled condition type {}", c.condition_type),
}
```

Exhaustive matches on `OrderCondition` need the new arm; the enum stays exhaustive so the compiler finds them. Serde uses the derived externally tagged form (`{"Unknown":{..}}`), like every other `OrderCondition` variant.

### 2. Historical `BarSize`, `Duration` and `WhatToShow` parse through `FromStr` only

`market_data::historical::BarSize`, `Duration` and `WhatToShow` each implemented `From<&str>` and `From<String>` beside their `FromStr`. The `From` impls were `Self::from_str(s).unwrap()`: an infallible conversion that panicked on any string `FromStr` rejects. Nothing in the crate, `examples/` or the integration crates called them. They are removed; `FromStr` is unchanged, so `s.parse()` accepts exactly the strings `From` did and returns `Err(HistoricalParseError)` where `From` panicked.

```rust,ignore
// 4.2 - infallible, panicked on an unrecognized string
let bar_size = BarSize::from("MIN5");
let duration: Duration = "1 D".into();
let what: WhatToShow = String::from("TRADES").into();

// 5.0 - Err(HistoricalParseError) on an unrecognized string
let bar_size: BarSize = "MIN5".parse()?;
let duration: Duration = "1 D".parse()?;
let what: WhatToShow = "TRADES".parse()?;
```

`ibapi::Error` now implements `From<HistoricalParseError>`, so the `?` above also works in a function returning `Result<_, ibapi::Error>`.

### 3. `Trade.tick_type` is removed

`market_data::realtime::Trade` carried `tick_type: String`, holding the wire code `"1"` (`Last`) or `"2"` (`AllLast`). It was constant per stream: `tick_by_tick(..).last()` only ever yields `"1"`, and `.all_last()` only `"2"`, so the method that opened the stream already names the feed. The field is removed.

A caller that merges both feeds into one stream tags each item at merge time instead:

```rust,ignore
// 4.2
if trade.tick_type == "2" { /* AllLast */ }

// 5.0 - tag at merge time
enum Feed { Last, AllLast }

let last = client.tick_by_tick(&contract, 0).last().await?.filter_data().map(|t| (Feed::Last, t));
let all = client.tick_by_tick(&contract, 0).all_last().await?.filter_data().map(|t| (Feed::AllLast, t));
let mut merged = futures::stream::select(last, all);
```

Consider whether you need both: `AllLast` is a superset of `Last`, adding the trades `Last` leaves out, and each subscription uses a tick-by-tick slot. Subscribing to `AllLast` alone and filtering on `special_conditions` avoids the merge.

### 4. The blocking client's `SharesChannel` marker trait is removed

`SharesChannel` was an empty trait (`pub trait SharesChannel {}`) reachable as `ibapi::subscriptions::SharesChannel` and `ibapi::client::blocking::SharesChannel`, meant to tag blocking subscriptions that share a channel keyed by message type rather than by request ID. It was inert: the only bound naming it was on a crate-private helper whose single caller already satisfied it. Its real function was to flag those subscriptions in rustdoc, and as a flag it misled: it marked `positions` and `news_bulletins` but not `account_updates`, `open_orders`, `all_open_orders`, `auto_open_orders` or `completed_orders`, which share channels the same way, so an unmarked type read as safe for concurrent use when it was not. It also marked `Vec<NewsProvider>`, a one-shot result rather than a subscription. The async client never had it. The trait, its three impls and both re-exports are gone.

What changes for compiling code:

- `use ibapi::subscriptions::SharesChannel;` and `use ibapi::client::blocking::SharesChannel;` no longer resolve. Delete the import.
- A downstream `impl SharesChannel for ...` or a `where Subscription<T>: SharesChannel` bound no longer compiles. Delete it; nothing in the crate depended on your impl, so nothing else needs to change.

Nothing changes at runtime. The hazards the trait could have flagged are gone with the fixes in this release (see [Behavioral changes](#behavioral-changes)): on the blocking client, live subscriptions of the same shared type each receive the whole stream, as on the async client; and on both clients, dropping one no longer cancels the stream for the others. The requests that share a channel still do, on both clients, and their responses still cannot be attributed to the request that caused them — see [Multi-Threading](../README.md#multi-threading) in the README.

### 5. Price, volume and percent-change conditions take `impl Into<ContractId>`

Every constructor for these three conditions now takes `contract_id: impl Into<ContractId>`: the `orders::builder::{price, volume, percent_change}` helpers, `PriceCondition::builder`, `VolumeCondition::builder` and `PercentChangeCondition::builder`, and the `new` functions on their builders. Before, `price` took `impl Into<i32>` and the others took `i32`. `impl Into<i32>` accepted anything with a `From` conversion into `i32`, such as `u8` / `u16`, `bool`, `OrderId` and the integer-coded order enums (`TriggerMethod`, `OcaType`, …), and none of those is a contract id. `ContractId` converts only from `i32`, so that decides what these functions accept.

Calls that pass an integer literal, an `i32` (for example `contract.contract_id`) or a `ContractId` still compile. The calls that no longer compile were passing the wrong value — most plausibly an order id where the contract id belonged:

```rust,ignore
// 4.2: compiled, and monitored whatever contract happened to have the order's id
let condition = price(order_id, "SMART").greater_than(150.0);

// 5.0: pass the id of the contract to monitor
let condition = price(contract.contract_id, "SMART").greater_than(150.0);
```

A narrower integer type tops out at 65535 at most, below most contract ids (AAPL's is `265598`); if you do hold one there, `i32::from(..)` it.

### 6. `Order` gains `preset_stop_loss_order_id` and `preset_profit_taker_order_id`

Two `Option<i32>` fields on `Order` request stop-loss / profit-taker children that TWS attaches from its order presets (#842). A struct literal that names every field without `..Default::default()` stops compiling; add the two fields as `None`, or finish the literal with `..Default::default()`. Both default to `None`, which sends the same request as before. Orders read back from TWS always have them `None`.

### 7. `SecurityIdType` gains `Unknown(String)` and loses `Copy`

`SecurityIdType` is decoded on every inbound `Contract`, including the `OpenOrder` and `ExecutionData` frames of `order_update_stream`. An identifier scheme outside the five modeled ones failed that decode with `Error::Parse`, and the stream ended (#840). It now follows `Rule80A` / `OrderOpenClose` (see [4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values)): an unrecognized non-empty value parses as `SecurityIdType::Unknown(raw)`, which `Display` / `ToField` write back unchanged. Empty input is still `Error::Parse`, and `FromStr` stays case-sensitive, so `"cusip"` is `Unknown("cusip")` rather than an error.

- **`Copy` is removed.** `.clone()` (or borrow) where code copied a `SecurityIdType` out of a `Contract`.
- **`as_str()` returns `&str`** instead of `&'static str`; for `Unknown` it borrows the raw value.
- **`#[non_exhaustive]` is removed.** The `Unknown` arm now absorbs new schemes, so matches can be exhaustive and the compiler finds them if a scheme is ever promoted to a typed variant. Wildcard arms still compile.
- **Serde and `utoipa`** keep the derives: `Unknown` serializes externally tagged (`{"Unknown":"WKN"}`), and the generated schema gains a matching `Unknown` object branch, as in [4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values).

### 8. `parser_registry` is removed

`ibapi::parser_registry` was a `#[doc(hidden)]` re-export of `messages::parser_registry`: the `MessageParser` trait, `MessageParserRegistry`, `FieldBasedParser`, `TimestampParser`, `parse_generic_message` and the `FieldDef` / `ParsedField` types, which mapped each message type to the field indices of the NUL-delimited text protocol. The crate has decoded protobuf frames only since 3.0 (#452), so the tables described a wire format it no longer supports. The module is removed, together with the `record_interactions` example that was its only caller and the `tws_interactions.yaml` that example had recorded.

There is no replacement. To capture traffic, set `IBAPI_RECORDING_DIR` (one file per message, responses re-framed after parsing) or `IBAPI_RAW_CAPTURE_DIR` (the inbound byte stream, length prefixes intact) — see [Debugging Techniques](troubleshooting.md#debugging-techniques) in the troubleshooting guide.

### 9. The `trace` module is removed

`ibapi::trace` (`Interaction`, `last_interaction`, `record_request`, `record_response`, and their `trace::blocking` forms) kept the most recent request and the text frames that answered it. The client stopped calling `record_request` when the transport went protobuf-only in 3.0 (#452), and a protobuf frame has no text form to record, so `last_interaction()` has returned `None` ever since. The module is removed, together with the `trace_test`, `async_trace_test` and `async_trace_test_simple` examples.

There is no replacement API. Capture traffic with `IBAPI_RECORDING_DIR` or `IBAPI_RAW_CAPTURE_DIR`, as in [§8](#8-parser_registry-is-removed), or set `RUST_LOG=ibapi=debug` for a log line per inbound message.

### 10. `SecurityType` parses through `FromStr`, and `ContractData` needs both submessages

`contracts::SecurityType` had an inherent `from(&str)` rather than `FromStr`, beside a hand-written `Display`, so `SecurityType::from` resolved differently from every other wire enum. It now takes the shape [4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values) gave the order enums:

- **`SecurityType` parses through `FromStr<Err = Error>`.** The inherent `from` is gone. The enum stays open: an unrecognized non-empty value parses as `Other(String)` carrying the raw wire value, which `Display` writes back unchanged, and there is no longer a `warn!` when that happens. Empty input is `Error::Parse`.
- **An absent or empty `sec_type` still decodes as `Other("")`.** `decode_contract` reads it through `parse_optional` and falls back to `Other(String::new())`, so an inbound `Contract` without one decodes exactly as in 4.2. The reference client's `decodeContract` sets `SecType` only `if (HasSecType)` and still delivers the frame, so absence is an unset field rather than a malformed frame — the same exception [4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values) records for `tif`. Requiring it would have ended `order_update_stream`, `positions` and every other stream carrying a contract on the first such frame, the failure [§7](#7-securityidtype-gains-unknownstring-and-loses-copy) fixed for `sec_id_type`. Only `"".parse::<SecurityType>()` is an error.
- **`ContractData` needs both submessages.** A `ContractData` frame with no `contract` or no `contract_details` fails to decode with `Error::Parse` naming the missing submessage (`missing in ContractData`), instead of yielding a default-constructed contract or details. The reference client (`EDecoder.cs`) drops such a frame; this crate has no skip channel, so `contract_details` returns the error, as an `OpenOrder` missing a submessage has since 4.2 ([4.0 guide §14](migration-4.0.md#14-order-enums-parse-through-fromstr-and-preserve-unrecognized-wire-values)).

```rust,ignore
// 4.2
let sec_type = SecurityType::from("STK");            // unknowns became Other(raw), with a warn!

// 5.0
let sec_type: SecurityType = "STK".parse()?;         // unknowns are Other(raw); "" is an error
```

What changes for compiling code:

- **`SecurityType::from(s)` no longer compiles on a `&str`**: the call now resolves to the blanket `From<SecurityType>`, so the compiler points at every site. Use `s.parse::<SecurityType>()?`.
- **`as_str()` is added**, returning `&str`; for `Other` it borrows the raw value.
- **`Display`, `ToField`, `Default` (`Stock`), serde and `utoipa` are unchanged.** The derives stay, so `Other` still serializes in the externally tagged form.

### 11. Unused `market_data` items are removed

- **`market_data::historical::WhatToShowParseError`** is gone. No API returns it, so no caller can hold one: `FromStr for WhatToShow` fails with `HistoricalParseError`, and `s.parse::<WhatToShow>()` is unchanged. Delete any `use`, impl or match arm naming it.
- **`market_data::realtime::BarSize`** and its prelude alias `RealtimeBarSize` are gone. No API has accepted it since 3.0 turned `realtime_bars` into a builder ([3.0 §7](migration-3.0.md#7-clientrealtime_bars-is-a-builder)). Real-time bars are always 5 seconds. Delete any `use` of either name. `market_data::historical::BarSize` and its alias `HistoricalBarSize` are unchanged.

### 12. `BracketOrderIds` converts from `Vec<i32>` through `TryFrom`

`From<Vec<i32>> for BracketOrderIds` asserted on the length, so an infallible conversion panicked on any `Vec` that did not hold exactly three ids. It is replaced by `TryFrom<Vec<i32>>`, which returns `ValidationError::InvalidBracketOrder` for any other length; `ibapi::Error` implements `From<ValidationError>`, so `?` works in a function returning `Result<_, ibapi::Error>`. `From<[i32; 3]>`, `BracketOrderIds::new` and the three public fields are unchanged.

```rust,ignore
// 4.2 - panicked unless ids.len() == 3
let bracket = BracketOrderIds::from(ids);

// 5.0 - Err(ValidationError::InvalidBracketOrder) unless ids.len() == 3
let bracket = BracketOrderIds::try_from(ids)?;
```

### 13. `OrderAnalysis` is removed

`orders::builder::OrderAnalysis` was the planned return type of `OrderBuilder::analyze()`, which has returned `orders::OrderState` since the builder was added (#311). No API produced or accepted it. Use `OrderState`: its `initial_margin_after`, `maintenance_margin_after`, `commission`, `commission_currency` and `warning_text` carry the same figures.

### 14. The `utoipa` feature is renamed `utoipa-6` and requires utoipa 6

With the feature enabled, public types derive `utoipa::ToSchema`, so utoipa is part of the public API. 5.0 moves from utoipa 5 to utoipa 6 and renames the feature after the utoipa major it targets. A later major will arrive as a new `utoipa-<major>` feature in a minor release, leaving `utoipa-6` in place; if several are enabled in one build, the newest wins. The two versions' `ToSchema` traits are distinct, so a crate still on utoipa 5 that names `ibapi` types in a `#[derive(OpenApi)]` or `#[schema(...)]` no longer compiles: our types implement utoipa 6's `ToSchema`, not the one it imports.

Rename the feature, and move your own utoipa dependency and any utoipa integration crates to versions built on utoipa 6:

```toml
# Cargo.toml
# 4.2
ibapi = { version = "4", features = ["utoipa"] }
utoipa = "5"

# 5.0
ibapi = { version = "5", features = ["utoipa-6"] }
utoipa = "6"
```

Enabling `utoipa` alone, as 4.x did, now fails to compile with a message naming `utoipa-6`: the name remains as an internal switch. utoipa 6 needs Rust 1.88 or later. Users without the feature are not affected.

### 15. `Error::ProtobufDecode` carries `errors::ProtobufDecodeError`

`Error::ProtobufDecode` held a `prost::DecodeError`, and `Error` implemented `From<prost::DecodeError>`. prost is pre-1.0, so each prost minor release would have been a breaking change to `ibapi` through this variant, its only public exposure. The payload is now the opaque `ibapi::errors::ProtobufDecodeError` (`Debug`, `Clone`, `Display`, `std::error::Error`, `Send + Sync`), and the `From` impl is removed.

- **The variant and its `Display` text are unchanged** (`protobuf decode error: ...`), so matching `Error::ProtobufDecode(_)` and printing the error still work.
- **Code that named `prost::DecodeError` from the payload, or converted one with `?` / `.into()`, no longer compiles.** Use the payload's `Display`, or map your own prost errors into your own error type (`Error::Simple(e.to_string())` if it must be an `ibapi::Error`).
- **`ProtobufDecodeError` has no public constructor**, so code outside the crate can't build an `Error::ProtobufDecode` (e.g. in tests); use another variant.
- **`source()` on `Error::ProtobufDecode` now returns `None`.** The decode detail was in both the message and the source, so chain printers (`anyhow`'s `{:#}`) showed it twice; it stays in the message.

```rust,ignore
// 4.2
if let Err(Error::ProtobufDecode(e)) = result {
    let e: prost::DecodeError = e;
    eprintln!("{e}");
}

// 5.0
if let Err(Error::ProtobufDecode(e)) = result {
    eprintln!("{e}"); // ibapi::errors::ProtobufDecodeError
}
```

`Error::ParseTime` keeps its `time::error::Parse` payload: `time` is part of the API through `OffsetDateTime` and `Date`, so wrapping its error would gain nothing.

### 16. Order methods take `impl Into<OrderId>`

`place_order`, `submit_order`, `cancel_order`, `order_builder::market_f_hedge`, `order_builder::bracket_order` and `OrderBuilder::parent` take `impl Into<OrderId>` instead of `i32`, so the ids in `BracketOrderIds` and `AttachedOrderIds` pass straight in. An `i32` value or literal still compiles. What breaks is an argument whose type was inferred from the old `i32` parameter: with a generic parameter, the compiler can no longer tell what to convert to.

```rust,ignore
// 4.2: the i32 parameter picked the target type
client.place_order(row_id.try_into()?, &contract, &order)?; // row_id: i64
client.cancel_order(text.parse().unwrap(), "")?;
let cancel = Client::cancel_order;

// 5.0: name the type
client.place_order(i32::try_from(row_id)?, &contract, &order)?;
client.cancel_order(text.parse::<i32>().unwrap(), "")?;
let cancel: fn(&Client, i32, &str) -> _ = Client::cancel_order;
```

The same applies to `.into()` from a narrower integer and to `Default::default()`. `OrderId::from(..)` works too.

### 17. `order_builder::auction_limit` and the `OrderType` auction variants are removed

Since 4.2 dropped `auction_limit`'s strategy parameter ([4.x guide §15](migration-4.0.md#15-orderbuilder-covers-the-integer-coded-order-enums-and-auctionstrategy-is-removed)), it has built the same `Order` as `limit_order`. Routing the contract to `BOX` is what makes it an auction order. `orders::builder::OrderType::AuctionLimit` and `OrderType::AuctionRelative` are removed for the same reason. They sent `LMT` and `REL`, the same as `OrderType::Limit` and `OrderType::Relative`, and validated the same way; serialized `OrderType` values naming them no longer deserialize. TWS still applies the account's configured auction strategy.

```rust,ignore
// 4.2
let order = auction_limit(Action::Buy, 10.0, 1.25);
let builder = builder.order_type(OrderType::AuctionLimit);

// 5.0
let order = limit_order(Action::Buy, 10.0, 1.25);
let builder = builder.order_type(OrderType::Limit);
```

### 18. `MarketDepths` gains `Reset`

A market-depth subscription used to yield code 317 ("Market depth data has been RESET") as `SubscriptionItem::Notice`, which `filter_data()` / `iter_data()` log and drop. A consumer on those adapters never saw the reset and merged the rebuilt rows into its stale book. The subscription now yields `MarketDepths::Reset` as data instead, so exhaustive matches on `MarketDepths` need the arm. On it, empty the book you hold; the rows that follow rebuild it. Code 316 (depth HALTED) still ends the stream with `Err` (#899).

```rust,ignore
// 5.0
match depth? {
    MarketDepths::MarketDepth(row) => apply(&mut book, row),
    MarketDepths::MarketDepthL2(row) => apply_l2(&mut book, row),
    MarketDepths::Reset => book.clear(),
}
```

If you matched `SubscriptionItem::Notice` with code 317 on a depth subscription, move that handling to the `Reset` arm.

### 19. `NoticeCategory::RequestError`; notice predicates follow `category()`

Every code in 200..=399 not claimed by an earlier rule was `NoticeCategory::OrderRejection`, although some are failed requests that have nothing to do with orders: 316 (depth HALTED), 354 (market data not subscribed), 366 (no historical query). Those codes, listed in `REQUEST_ERROR_CODES`, are now `NoticeCategory::RequestError`, with `Notice::is_request_error()`. They still end the request, as before. Codes that also answer orders stay `OrderRejection`: 200 (no security definition) and 320-323 (server errors on a request). `NoticeCategory` is `#[non_exhaustive]`, so a match needs no new arm, but a wildcard arm will now see these codes (#898).

`Notice::is_warning()` and `Notice::is_order_rejection()` tested a code range; every `is_*` category predicate is now `category() == X` for its variant `X`, so they never overlap:

| Predicate | No longer `true` for |
|---|---|
| `is_warning()` | 2188 (a `DataAdvisory`) |
| `is_order_rejection()` | 202 (`Cancellation`), 317 (`DataAdvisory`), 399 with a `Warning:` line (`Warning`), `REQUEST_ERROR_CODES` (`RequestError`) |

If you need the old range test, use the constant:

```rust,ignore
// 4.x
if notice.is_order_rejection() { /* any 200..=399 */ }

// 5.0
if ibapi::ORDER_REJECTION_CODE_RANGE.contains(&notice.code) { /* any 200..=399 */ }
// or, usually what was meant:
if notice.is_order_rejection() || notice.is_request_error() { /* failed */ }
```

### 20. `Client::next_request_id()` is removed

`next_request_id()` (blocking and async) handed out a fresh request id that no request used. Nothing in the API accepts a caller-chosen request id, so the value had no use: passing it to `cancel_contract_details` or `cancel_historical_ticks` cancelled nothing. The ids of requests in flight come from `Subscription::request_id()` and `ContractDetailsBuilder::request_id()`, both unchanged, and the new `TickSubscription::request_id()`. Delete any call; to cancel a request, drop its subscription or call `cancel()` on it.

### 21. `SpreadBuilder` takes `impl Into<ContractId>`, and `iron_condor` is removed

`SpreadBuilder::add_leg`, `calendar` and `vertical` take `impl Into<ContractId>` instead of `i32`, as the condition constructors do ([§5](#5-price-volume-and-percent-change-conditions-take-impl-intocontractid)). Integer literals, `i32` values (for example `contract.contract_id`) and `ContractId` all compile. An argument converted with `.into()`, `.try_into()` or `.parse()` that relied on the old `i32` parameter to pick its type now needs the type named.

`SpreadBuilder::iron_condor(long_put, short_put, short_call, long_call)` is removed. It took four contract ids of one type in a fixed order, so swapping two compiled and built a different strategy. Chain two verticals instead, each long the wing and short the body:

```rust,ignore
// 4.2
let spread = Contract::spread().iron_condor(long_put, short_put, short_call, long_call).build()?;

// 5.0
let spread = Contract::spread()
    .vertical(long_put, short_put)
    .vertical(long_call, short_call)
    .build()?;
```

The legs come out as buy long put, sell short put, buy long call, sell short call; `iron_condor` put the call legs the other way round. In a paper what-if, TWS accepted both orders with the same order state. To keep the old order, add the legs with `add_leg`.

### 22. Fluent trailing stops take `TrailBy`, and `OrderBuilder` is generic over its target

`OrderBuilder::trailing_stop` and `trailing_stop_limit` take a `orders::builder::TrailBy` instead of an `f64` percent, so a trailing stop can trail by a fixed amount as well as a percentage. `TrailBy::Percent` is the old behavior and sets `trailing_percent`; `TrailBy::Amount` sets `aux_price`, the form `order_builder::trailing_stop_limit` builds. A bare `f64` no longer compiles:

```rust,ignore
// 4.2
client.order(&contract).sell(100).trailing_stop(5.0, 95.0).submit()?;
client.order(&contract).sell(100).trailing_stop_limit(5.0, 95.0, 0.5).submit()?;

// 5.0
use ibapi::orders::builder::TrailBy;

client.order(&contract).sell(100).trailing_stop(TrailBy::Percent(5.0), 95.0).submit()?;
client.order(&contract).sell(100).trailing_stop_limit(TrailBy::Percent(5.0), 95.0, 0.5).submit()?;
```

`OrderBuilder<'a, C>` is now `OrderBuilder<T>`, where `T` is `orders::ClientBound<'a, C>` for the builder `Client::order` returns and `orders::Detached` for the new `Order::builder()`, which builds an `Order` without a client. `BracketOrderBuilder<'a, C>` is unchanged. This only affects code that names the type, for example a function returning `OrderBuilder<'a, Client>`: write `OrderBuilder<ClientBound<'a, Client>>`. Chains starting from `client.order(..)` are unchanged.

## Behavioral changes

No code changes required, but observable at runtime:

- **Shared streams are cancelled by their last subscriber.** `positions`, `account_updates` and `news_bulletins` are one stream per client at TWS, and their cancel carries no id. Dropping one of several live subscriptions to the same stream used to send that cancel and end the stream for the others; both clients now send it only when the last one ends. An async `Subscription` can be dropped on any thread, including one with no Tokio runtime; its cancel still goes out.
- **Blocking shared subscriptions each receive the whole stream.** On the blocking client, live subscriptions of the same shared type (`positions`, `open_orders`, ...) used to read one queue, so each response reached whichever of them read it first. Every live subscription of the type now receives every response, as on the async client, and a new subscription no longer starts with items left over from before it was made. Responses still cannot be attributed to the request that caused them — see [Multi-Threading](../README.md#multi-threading).
- **A position frame without its contract is an error.** `Position` (`positions`), `PositionMulti` (`positions_multi`) and `PortfolioValue` (`account_updates`) decoded a frame with no `contract` submessage as `Contract::default()` — a US stock with contract id `0` on `SMART` in `USD`, indistinguishable from a real position. Each now fails with `Error::Parse` whose field is `contract` and whose reason names the message (`missing in Position` and so on), as `OpenOrder`, `CompletedOrder` and `ExecutionDetails` have since #829; the subscription yields the error and ends, as for any decode error. A `PortfolioValue` arriving during the connection handshake is decoded only when a startup callback is installed, and a failure there becomes a `HANDSHAKE_DECODE_FAILURE_CODE` notice on the notice stream (at the first connect, only a stream pre-bound with `ClientBuilder::connect_with_notice_stream` sees it). The reference clients do the same for these three messages: Java `EDecoder.processPositionMsgProtoBuf`, `processPositionMultiMsgProtoBuf` and `processPortfolioValueMsgProtoBuf` (and the Python `decoder.py` equivalents, TWS API 10.50) return before the typed callback when the contract is absent. `SymbolSamples` and `ScannerData` are unchanged, still decoding a contract-less row with `Contract::default()`, because the reference clients still deliver those rows; `ContractData` is covered in [§10](#10-securitytype-parses-through-fromstr-and-contractdata-needs-both-submessages).
- **`Subscription::cancel()` always ends the subscription.** It used to end only subscriptions it wrote a TWS cancel for; order subscriptions (`place_order`, `submit_order`, `cancel_order`, `exercise_options`), `order_update_stream`, `executions`, `option_chain` and streams that had already ended kept delivering until dropped. On the blocking client, the subscription now yields what was already queued, then `Err(Error::Cancelled)`, then ends. On the async client, every clone yields `Err(Error::Cancelled)` at its next poll, then ends. Cancelling an order subscription still does not cancel the order at TWS: use `cancel_order` (#911).
- **Request ids start at 1,500,000,000.** `Subscription::request_id()` and `ContractDetailsBuilder::request_id()` return values at or above 1,500,000,000 instead of 9000-based ones, and order ids must stay below it: `place_order`, `submit_order` and `cancel_order` return `Error::OrderIdInRequestRange` for an order id at or above it, including the `parent_id`, `preset_stop_loss_order_id` and `preset_profit_taker_order_id` it carries, and so do `exercise_options` and `next_valid_order_id` when the next order id reaches it. Request ids and order ids used to share one range, and an error frame carries a single id, so an order's cancellation or rejection could be delivered to a data subscription holding the same number, and a late error for a finished request could appear on the order-update stream as if it belonged to an order. Routing now decides by range. Ids from `next_order_id()` and `next_valid_order_id()` are unaffected unless your account's sequence has reached the floor, which also makes `connect` fail with `Error::ConnectionRejected`. If you number orders yourself, keep the numbers below 1,500,000,000 (#789).
- **An unknown message id during the handshake raises `UNKNOWN_MESSAGE_TYPE_CODE`.** A handshake frame whose message id maps to no `IncomingMessages` kind raised `HANDSHAKE_UNKNOWN_FRAME_CODE` (-3); it now raises `UNKNOWN_MESSAGE_TYPE_CODE` (-5), the code the same frame gets after the handshake, so a desynchronized stream reads the same whether or not a reconnect is in flight. `is_handshake_synthetic()` is false for it. `HANDSHAKE_UNKNOWN_FRAME_CODE` still covers a recognized kind with no `StartupMessage` variant, and its text no longer carries the message id. The new `Notice::is_client_synthesized()` is true for every client-synthesized code (any negative code) (#906).
- **Async shutdown ends subscriptions with `Err(Error::Shutdown)`.** On the async client, `disconnect()` or dropping the `Client` used to end live subscriptions (including `order_update_stream`) with no error, so a stream ended by shutdown looked like one that closed normally. Each now yields `Err(Error::Shutdown)`, then ends, as on the blocking client. Code that took the end of the stream as the shutdown signal now sees the error first; `notice_stream` still just ends (#895).
- **`OrderBuilder::build()` sends only the price fields the order type uses.** Each order-type setter writes only its own fields, so a field from an earlier one used to reach the order: `.trailing_stop(TrailBy::Percent(5.0), 95.0).limit(100.0)` sent a limit order with `trailing_percent` set, `.stop_limit(..)` followed by `.trailing_stop_limit(..)` kept the stop-limit's `limit_price`, and a trail amount survived as `aux_price`. `limit_price`, `aux_price`, `trailing_percent`, `trail_stop_price` and `limit_price_offset` are now sent only for the types that use them; the last setter's type decides. `.order_type(MarketIfTouched | LimitIfTouched | Relative | PassiveRelative | PeggedToMarket)` with no trigger or offset fails with `ValidationError::MissingRequiredField("aux_price")` instead of building an order TWS would reject; use the named setter (`.market_if_touched(price)` and so on) (#901).

## Quick migration checklist

1. Add an `OrderCondition::Unknown(c)` arm to exhaustive matches on order conditions, and replace `OrderCondition::from(code)` with a condition builder — see [§1](#1-ordercondition-gains-unknownunknowncondition).
2. Replace `BarSize::from(s)`, `Duration::from(s)` and `WhatToShow::from(s)` (and `.into()` to those types) with `s.parse()?` — see [§2](#2-historical-barsize-duration-and-whattoshow-parse-through-fromstr-only).
3. Drop reads of `Trade.tick_type`; if you merge the `last()` and `all_last()` streams, tag each item when merging — see [§3](#3-tradetick_type-is-removed).
4. Delete any `use ...::SharesChannel` import and any `impl SharesChannel for ...` or `Subscription<T>: SharesChannel` bound — see [§4](#4-the-blocking-clients-shareschannel-marker-trait-is-removed).
5. Pass an `i32` or `ContractId` as `contract_id` to the price, volume and percent-change condition constructors; an `OrderId`, enum or narrower integer there was a bug — see [§5](#5-price-volume-and-percent-change-conditions-take-impl-intocontractid).
6. Add `preset_stop_loss_order_id: None, preset_profit_taker_order_id: None` to exhaustive `Order` struct literals, or end them with `..Default::default()` — see [§6](#6-order-gains-preset_stop_loss_order_id-and-preset_profit_taker_order_id).
7. Add `.clone()` where code relied on `SecurityIdType: Copy`, and handle `SecurityIdType::Unknown(raw)` where you match on it — see [§7](#7-securityidtype-gains-unknownstring-and-loses-copy).
8. Drop any `use ibapi::parser_registry` import; the module is gone and capture goes through `IBAPI_RECORDING_DIR` / `IBAPI_RAW_CAPTURE_DIR` instead — see [§8](#8-parser_registry-is-removed).
9. Delete calls into `ibapi::trace`; `last_interaction()` has returned `None` since 3.0 — see [§9](#9-the-trace-module-is-removed).
10. Replace `SecurityType::from(s)` with `s.parse::<SecurityType>()?` — see [§10](#10-securitytype-parses-through-fromstr-and-contractdata-needs-both-submessages).
11. Remove references to `historical::WhatToShowParseError`, `realtime::BarSize` and `RealtimeBarSize` — see [§11](#11-unused-market_data-items-are-removed).
12. Replace `BracketOrderIds::from(vec)` with `BracketOrderIds::try_from(vec)?` — see [§12](#12-bracketorderids-converts-from-veci32-through-tryfrom).
13. Replace `orders::builder::OrderAnalysis` with `orders::OrderState` — see [§13](#13-orderanalysis-is-removed).
14. If you enable the `utoipa` feature, rename it to `utoipa-6` and upgrade to utoipa 6 — see [§14](#14-the-utoipa-feature-is-renamed-utoipa-6-and-requires-utoipa-6).
15. Replace uses of `prost::DecodeError` from `Error::ProtobufDecode`, and any `From<prost::DecodeError> for ibapi::Error` conversion — see [§15](#15-errorprotobufdecode-carries-errorsprotobufdecodeerror).
16. Where an order id argument is converted with `.try_into()`, `.into()`, `.parse()` or `Default::default()`, name the target type (`i32::try_from(..)`, `.parse::<i32>()`) — see [§16](#16-order-methods-take-impl-intoorderid).
17. Replace `auction_limit(..)` with `limit_order(..)` (same arguments), and `OrderType::AuctionLimit` / `AuctionRelative` with `OrderType::Limit` / `Relative` — see [§17](#17-order_builderauction_limit-and-the-ordertype-auction-variants-are-removed).
18. Add a `MarketDepths::Reset` arm that empties your book to exhaustive matches on `MarketDepths`, and drop any code-317 notice handling on depth subscriptions — see [§18](#18-marketdepths-gains-reset).
19. Check code that treats `NoticeCategory::OrderRejection` or `is_order_rejection()` as "any 200..=399 failure": request errors now come as `RequestError` / `is_request_error()`, and `is_warning()` is false for 2188 — see [§19](#19-noticecategoryrequesterror-notice-predicates-follow-category).
20. Delete calls to `Client::next_request_id()`; take a live request's id from `Subscription::request_id()`, `ContractDetailsBuilder::request_id()` or `TickSubscription::request_id()` — see [§20](#20-clientnext_request_id-is-removed).
21. Replace `SpreadBuilder::iron_condor(lp, sp, sc, lc)` with `.vertical(lp, sp).vertical(lc, sc)`, and name the target type where a spread leg's contract id is converted with `.into()`, `.try_into()` or `.parse()` — see [§21](#21-spreadbuilder-takes-impl-intocontractid-and-iron_condor-is-removed).
22. Wrap the trail in `TrailBy::Percent(..)` in `OrderBuilder::trailing_stop` / `trailing_stop_limit` calls, and write `OrderBuilder<ClientBound<'a, C>>` where you named `OrderBuilder<'a, C>` — see [§22](#22-fluent-trailing-stops-take-trailby-and-orderbuilder-is-generic-over-its-target).
23. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and your test suite for each feature flag you support.

## Need help?

- Examples: `examples/async` and `examples/sync`
- README: [Handling notifications](../README.md#handling-notifications)
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
