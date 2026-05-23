# Migration Guide: 2.x to 3.0

Version 3.0 is a breaking release. This guide walks through the changes required to upgrade from `ibapi` 2.x to 3.0. For 1.x → 2.x, see [`MIGRATION.md`](../MIGRATION.md).

## Highlights

- `Subscription<T>::next()` now returns `Option<Result<SubscriptionItem<T>, Error>>`. The new `SubscriptionItem<T>` enum has two arms: `Data(T)` for decoded payloads and `Notice(Notice)` for non-fatal IB notices that share the subscription's `request_id`.
- `Subscription::error()` and `Subscription::clear_error()` are removed. Terminal errors surface as `Some(Err(_))` on the next call to `next()`; subsequent calls return `None`.
- New `Client::notice_stream()` exposes globally routed IB notices (connectivity codes 1100/1101/1102, farm-status 2104/2105/2106/2107/2108, etc.) that are not tied to any subscription.
- `Client::builder()` is the canonical entry point, replacing `connect_with_callback` / `connect_with_options`. Two terminals: `.connect()` and `.connect_with_notice_stream()`. `ConnectionOptions`, `StartupMessageCallback`, and `StartupNoticeCallback` are removed.
- Per-T `Notice`/`Message` variants on `PlaceOrder`, `OrderUpdate`, etc. are gone — notices route through `SubscriptionItem::Notice` and `Client::notice_stream()`.
- `OrderStatus.status` and `OrderState.status` are now typed as [`OrderStatusKind`](https://docs.rs/ibapi/latest/ibapi/orders/enum.OrderStatusKind.html) (a strict 9-variant enum) instead of `String`. New `is_active()` / `is_terminal()` helpers replace magic-string compares.
- The text wire protocol is gone; v3.0 is protobuf-only and requires a TWS/IB Gateway with server version **210 (`PROTOBUF_SCAN_DATA`) or later**. Older servers are rejected with `Error::ServerVersion` immediately after the handshake.
- `Contract` is `#[non_exhaustive]`. Build via the typed entry points (`Contract::stock`/`call`/`put`/`futures`/`forex`/`crypto`/`index`/`bond_*`/`spread`) or `ContractBuilder::new()` for the field-minimal escape hatch. Bare `Contract { … ..Default::default() }` no longer compiles outside the crate.

## Notification handling: the new shape

In 2.x, a sync `Subscription<T>` yielded bare `T` values via iterator and tracked the most recent error in a side-channel `error()` accessor. Warnings were log-only.

In 3.0, every consumer sees the same envelope:

```text
Option<Result<SubscriptionItem<T>, Error>>
       │             ├─ Data(T)
       │             └─ Notice(Notice)   // request-scoped, stream stays open
       └─ Err(Error)                     // terminal; next() returns None after this
```

If you don't care about notices, two convenience accessors ship per side:

| Side  | Data only                         | Data + notices               |
|-------|-----------------------------------|------------------------------|
| Sync  | `iter_data()` / `next_data()`     | `iter()` / `next()`          |
| Async | `data_stream()` / `next_data()`   | `stream()` / `next()`        |

Filtered notices are logged at `warn!`, so dropping to `iter_data()` does not silently swallow problems.

## Breaking changes

### 1. `Subscription::next()` shape change (sync)

```rust,ignore
// v2.x — sync iteration over bare T
for bar in &subscription {
    println!("bar: {bar:?}");
}
if let Some(err) = subscription.error() {
    eprintln!("subscription error: {err}");
}
```

Mechanical migration — drop notices, keep data:

```rust,ignore
// v3.0 — data-only iteration; notices are filtered (logged at warn!).
// `iter_data()` yields Result<T, Error>, so handle the Err arm explicitly —
// `.flatten()` would silently drop terminal errors.
for item in subscription.iter_data() {
    match item {
        Ok(bar) => println!("bar: {bar:?}"),
        Err(e)  => { eprintln!("error: {e}"); break; }
    }
}
```

Or pattern-match if you want full visibility:

```rust,ignore
// v3.0 — data + notices + errors
for item in &subscription {
    match item {
        Ok(SubscriptionItem::Data(bar))    => println!("bar: {bar:?}"),
        Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
        Err(e)                             => { eprintln!("error: {e}"); break; }
    }
}
```

### 2. `Subscription::error()` and `clear_error()` removed

The side-channel error accessor is gone. Errors flow through `next()` like any other terminal item:

```rust,ignore
// v2.x
for bar in &subscription { /* ... */ }
match subscription.error() {
    Some(Error::ConnectionReset) => /* retry */ {},
    Some(other) => eprintln!("error: {other}"),
    None => {}
}

// v3.0 — inspect the Err variant directly
for item in &subscription {
    match item {
        Ok(SubscriptionItem::Data(bar)) => { /* ... */ }
        Ok(SubscriptionItem::Notice(_)) => {}
        Err(Error::ConnectionReset) => /* retry */ break,
        Err(e) => { eprintln!("error: {e}"); break; }
    }
}
```

### 3. Async `Subscription<T>` wraps `T`

```rust,ignore
// v2.x — async stream over bare T
while let Some(bar) = subscription.next().await {
    println!("bar: {bar:?}");
}
```

```rust,ignore
// v3.0 — data only
while let Some(Ok(bar)) = subscription.next_data().await {
    println!("bar: {bar:?}");
}

// v3.0 — full envelope
while let Some(item) = subscription.next().await {
    match item {
        Ok(SubscriptionItem::Data(bar))    => println!("bar: {bar:?}"),
        Ok(SubscriptionItem::Notice(note)) => eprintln!("notice: {note}"),
        Err(e)                             => { eprintln!("error: {e}"); break; }
    }
}
```

`subscription.stream()` and `subscription.data_stream()` provide `futures::Stream` adapters with the same data/full split.

### 4. Per-T `Notice`/`Message` variants removed

`PlaceOrder::Message`, `OrderUpdate::Message`, and the analogous per-T notice variants on other subscription enums are gone. Per-subscription notices now arrive as `SubscriptionItem::Notice(_)`; globally routed notices go through `Client::notice_stream()`. If you matched on the typed variant, drop that arm and migrate to the envelope or the global stream — see §1, §3 above.

### 5. `OrderStatus.status` / `OrderState.status` typed as `OrderStatusKind`

Both fields were `String` in 2.x. In 3.0 they are typed as `OrderStatusKind`, a strict 9-variant enum (`ApiPending`, `PendingSubmit`, `PendingCancel`, `PreSubmitted`, `Submitted`, `ApiCancelled`, `Cancelled`, `Filled`, `Inactive`) matching IB's canonical OrderStatus vocabulary.

```rust,ignore
// v2.x — magic-string compare
if status.status == "Filled" || status.status == "Cancelled" {
    break;
}

// v3.0 — typed predicate; covers Filled/Cancelled/ApiCancelled/Inactive
if status.status.is_terminal() {
    break;
}
```

`is_active()` covers `PreSubmitted`, `PendingSubmit`, `PendingCancel`, `Submitted`. The two helpers together cover 8 of 9 variants; `ApiPending` is neither active nor terminal — do not assume `!is_active() ⇒ is_terminal()`.

`OrderStatusKind` implements `Display` (round-trips back to the IB string) and `FromStr`. The decoder propagates `Error::Parse` if TWS sends an unknown, empty, or missing status string — incomplete responses fail loudly rather than silently defaulting to `Submitted`. If your 2.x code treated `status: ""` as "no status yet," handle the new `Err(Error::Parse(_))` arm on the subscription instead.

`OrderState.completed_status` stays `String` — TWS uses that field for free-form descriptions like `"Cancelled by Trader"` or `"Filled Size: 1"`, not enum values.

If you compared against the wire string, replace it with the matching variant:

```rust,ignore
// v2.x
match status.status.as_str() {
    "Submitted"    => log_submitted(),
    "Filled"       => finalize(),
    "Cancelled"    => rollback(),
    other          => log_unknown(other),
}

// v3.0
use ibapi::orders::OrderStatusKind;
match status.status {
    OrderStatusKind::Submitted => log_submitted(),
    OrderStatusKind::Filled    => finalize(),
    OrderStatusKind::Cancelled => rollback(),
    other                      => log_other(other),
}
```

`Display`-format strings still match the IB wire vocabulary, so `format!("{}", status.status)` and `status.status.to_string()` produce the same values you saw in 2.x.

### 6. `ResponseMessage` is crate-private

`ResponseMessage` was a low-level wire envelope; in 3.0 it is `pub(crate)` and no longer re-exported. See ["StartupMessage::Other removed; ResponseMessage is crate-private"](#startupmessageother-removed-responsemessage-is-crate-private) for the route to handshake-time observability that replaces the previous escape hatches.

### 7. `Client::realtime_bars` is a builder

The flat `realtime_bars(&contract, bar_size, what_to_show, trading_hours, [options])` form is gone on both sync and async. `Client::realtime_bars(&contract)` now returns a `RealtimeBarsBuilder`; configure with chained methods and finish with `.subscribe()`.

```rust,ignore
// v2.x (sync) — 4 args, no options parameter
let sub = client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Extended)?;

// v2.x (async) — 5 args, references on enums, options required
let sub = client.realtime_bars(&contract, &BarSize::Sec5, &WhatToShow::Trades, TradingHours::Extended, vec![]).await?;
```

```rust,ignore
// v3.0 (sync)
let sub = client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe()?;

// v3.0 (async)
let sub = client.realtime_bars(&contract).trading_hours(TradingHours::Extended).subscribe().await?;
```

Defaults: `WhatToShow::Trades`, `TradingHours::Regular`, no extra options. Chain `.what_to_show(_)`, `.trading_hours(_)`, `.options(_)` to override. The reserved `options: Vec<TagValue>` parameter — previously async-only — is now reachable from the sync side too, fixing the asymmetry called out in #486. The `BarSize` parameter is gone: TWS only accepts 5-second bars on the wire, so it never had per-call meaning. A `.bar_size(...)` method can be added non-breakingly if IB ever expands support.

### 8. `Contract` is `#[non_exhaustive]`

`Contract` has gained `#[non_exhaustive]`, so external crates can no longer build it via struct literal syntax. The typed entry points on `Contract` are the canonical path; the field-minimal `ContractBuilder::new()` is the escape hatch when no typed builder fits.

```rust,ignore
// v2.x — no longer compiles outside the crate
let c = Contract {
    symbol: Symbol::from("AAPL"),
    security_type: SecurityType::Stock,
    exchange: Exchange::from("SMART"),
    currency: Currency::from("USD"),
    ..Default::default()
};
```

```rust,ignore
// v3.0 — typed entry points cover the common cases
let stock  = Contract::stock("AAPL").build();
let call   = Contract::call("AAPL").strike(150.0).expires_on(2024, 12, 20).build();
let put    = Contract::put("SPY").strike(450.0).expires_weekly().build();
let future = Contract::futures("ES").front_month().build();
let forex  = Contract::forex("EUR", "USD").build();
let crypto = Contract::crypto("BTC").build();
let index  = Contract::index("SPX");
let bond   = Contract::bond_cusip("912810RN0");

// Field-minimal escape hatch — anything spellable as `Contract { ... }`
// stays spellable here. Required only when no typed builder exists for
// your security type (warrants, exotic instruments, contract_id-only
// lookups, etc.).
let warrant = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Warrant)
    .exchange("SMART")
    .currency("USD")
    .build()?;
```

**Escape-hatch invariant.** Every `pub` field on `Contract` is settable on `ContractBuilder` (including `last_trade_date`, which servers overwrite on contract-details round-trips). A regression test at `src/contracts/common/contract_builder/tests.rs::setter_parity_with_contract_fields` enforces this — when a new `Contract` field lands without a corresponding setter, the test fails to compile.

The wrapper types `Symbol`, `Exchange`, `Currency` now implement `PartialEq<str>` / `PartialEq<&str>` (both directions), so `contract.symbol == "AAPL"` works without `.as_str()`.

### 9. `ComboLeg.action` typed as `LegAction`

`ComboLeg.action` was `String` in 2.x. In 3.0 it is typed as `LegAction`, a strict 3-variant enum (`Buy`, `Sell`, `SellShort`) matching IBKR's combo-leg wire vocabulary. `LegAction` already existed as the `SpreadBuilder::add_leg(_, LegAction)` parameter type; 3.0 reuses it as the struct field and adds the `SellShort` variant.

`SLONG` is intentionally excluded — combo legs do not accept it (only the `SSHORT_COMBO_LEGS` gate exists in the C# reference at server version 35, well below our floor of 210; no `SLONG` gate exists for combo legs). If you need long-undelivered semantics, that's the outer `Order.action: Action::SellLong`, not a combo leg.

```rust,ignore
// v2.x
let leg = ComboLeg {
    contract_id: 12345,
    action: "BUY".to_string(),
    ..Default::default()
};

// v3.0
let leg = ComboLeg {
    contract_id: 12345,
    action: LegAction::Buy,
    ..Default::default()
};
```

`LegAction` implements `Display` (`"BUY"` / `"SELL"` / `"SSHORT"`) and `FromStr<Err = Error>`. The decoder propagates `Error::Parse` if TWS sends an empty or unknown action — silent fallback to `LegAction::default()` is off the table.

### 10. `Contract.right` typed as `Option<OptionRight>`

`Contract.right` was `String` in 2.x (empty string meant "no right"). In 3.0 it is typed as `Option<OptionRight>` — `None` on non-option contracts, `Some(OptionRight::Call)` or `Some(OptionRight::Put)` on options. The decoder rejects unknown wire values as `Error::Parse` rather than silently storing them as raw strings.

`OptionRight` is `#[non_exhaustive]` and implements `Display` (`"C"` / `"P"`) and `FromStr<Err = Error>`. It is case-sensitive and accepts only the canonical single-character form; lowercase and the historical long forms (`"CALL"` / `"PUT"`) now produce `Err`.

`Contract::option`'s 4th parameter changes from `&str` to `OptionRight`. `ContractBuilder::right()` changes from `impl Into<String>` to `OptionRight`. The builder's runtime "right must be P or C" validation has been removed — invalid rights are structurally unrepresentable.

```rust,ignore
// v2.x
let call = Contract::option("AAPL", "20240119", 150.0, "C");
assert_eq!(call.right, "C");

let builder_call = ContractBuilder::option("AAPL", "SMART", "USD")
    .strike(150.0)
    .right("C")
    .build()?;

// v3.0
use ibapi::contracts::OptionRight;

let call = Contract::option("AAPL", "20240119", 150.0, OptionRight::Call);
assert_eq!(call.right, Some(OptionRight::Call));

let builder_call = ContractBuilder::option("AAPL", "SMART", "USD")
    .strike(150.0)
    .right(OptionRight::Call)
    .build()?;
```

If you match on the field, swap `if contract.right == "C"` for `if contract.right == Some(OptionRight::Call)`. To emit the wire string, call `.as_str()` (`OptionRight::Call.as_str() == "C"`).

### 11. `Contract.security_id_type` typed as `Option<SecurityIdType>`

`Contract.security_id_type` was `String` in 2.x (empty string meant "no identifier scheme"). In 3.0 it is typed as `Option<SecurityIdType>` — `None` on contracts without an external identifier, `Some(SecurityIdType::Isin)` / `::Cusip` / `::Sedol` / `::Ric` / `::Figi` when one is paired with `security_id`. The decoder rejects unknown wire values as `Error::Parse` rather than silently storing them as raw strings.

`SecurityIdType` is `#[non_exhaustive]` (IBKR's catalogue grows over time) and implements `Display` returning the canonical uppercase wire string and `FromStr<Err = Error>`. `FromStr` is case-sensitive; lowercase forms now produce `Err`.

`ContractBuilder::security_id_type()` changes from `impl Into<String>` to `SecurityIdType`. `Contract::bond_cusip` / `Contract::bond_isin` and `Contract::bond(BondIdentifier::*)` continue to set the field internally — no caller-side change for the bond constructors.

```rust,ignore
// v2.x
let bond = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Stock)
    .exchange("SMART")
    .currency("USD")
    .security_id_type("ISIN")
    .security_id("US0378331005")
    .build()?;
assert_eq!(bond.security_id_type, "ISIN");

// v3.0
use ibapi::contracts::SecurityIdType;

let bond = ContractBuilder::new()
    .symbol("AAPL")
    .security_type(SecurityType::Stock)
    .exchange("SMART")
    .currency("USD")
    .security_id_type(SecurityIdType::Isin)
    .security_id("US0378331005")
    .build()?;
assert_eq!(bond.security_id_type, Some(SecurityIdType::Isin));
```

If you match on the field, swap `if contract.security_id_type == "ISIN"` for `if contract.security_id_type == Some(SecurityIdType::Isin)`. To emit the wire string, call `.as_str()` (`SecurityIdType::Isin.as_str() == "ISIN"`).

### 12. `ExecutionFilter.side` typed as `Option<ExecutionFilterSide>`

`ExecutionFilter.side` was `String` in 2.x (empty string meant "no filter"). In 3.0 it is typed as `Option<ExecutionFilterSide>` — `None` for no filter, `Some(ExecutionFilterSide::Buy)` or `Some(ExecutionFilterSide::Sell)` to restrict the response. Invalid filter values are no longer expressible — they fail at compile time rather than at the server.

`ExecutionFilterSide` is `#[non_exhaustive]` and implements `Display` (`"BUY"` / `"SELL"`) and `FromStr<Err = Error>`. `FromStr` is case-sensitive.

**Note: distinct from [`Action`].** `Action` covers the outbound order-side vocabulary (including `SellShort` / `SellLong`), neither accepted on the filter. A subset enum here prevents constructing filter values the server rejects.

```rust,ignore
// v2.x
let filter = ExecutionFilter {
    side: "BUY".to_owned(),
    ..ExecutionFilter::default()
};

// v3.0
use ibapi::orders::ExecutionFilterSide;

let filter = ExecutionFilter {
    side: Some(ExecutionFilterSide::Buy),
    ..ExecutionFilter::default()
};
```

If you match on the field, swap `if filter.side == "BUY"` for `if filter.side == Some(ExecutionFilterSide::Buy)`.

### 13. `Subscription` import path consolidation

`ibapi::client::Subscription` was a duplicate re-export of `ibapi::subscriptions::Subscription`. In 3.0 it has been removed; the canonical path is `ibapi::subscriptions::Subscription` (or `use ibapi::prelude::*;` for the convenience re-export). The labelled sync-explicit path `ibapi::client::blocking::Subscription` is unchanged — use it when you need the sync `Subscription<T>` while both `sync` and `async` features are enabled.

```rust,ignore
// v2.x / pre-PR
use ibapi::client::Subscription;

// v3.0
use ibapi::subscriptions::Subscription;
// or
use ibapi::prelude::*;
```

### 14. `SharesChannel` import path consolidation

Mirror of §13 for the `SharesChannel` marker trait. `ibapi::client::SharesChannel` and `ibapi::client::sync::SharesChannel` were duplicate re-exports of `ibapi::subscriptions::SharesChannel`. In 3.0 both are removed; the canonical path is `ibapi::subscriptions::SharesChannel`. The labelled sync-explicit path `ibapi::client::blocking::SharesChannel` is unchanged.

```rust,ignore
// v2.x / pre-PR
use ibapi::client::SharesChannel;

// v3.0
use ibapi::subscriptions::SharesChannel;
```

### 15. Historical data: sync/async parity

The async historical API has been reshaped to mirror the sync surface; `what_to_show` is now required on both `historical_data` and `historical_data_streaming` (issue #210). The single async `historical_schedule(contract, Option<OffsetDateTime>, duration)` has been split into the two named methods sync already used, and the `interval_end` parameter on the sync side was renamed to `end_date` for consistency with the wire field name.

**`historical_data` / `historical_data_streaming` — `what_to_show` is no longer optional:**

```rust,ignore
// v2.x — async wrapped what_to_show in Option
client.historical_data(
    &contract, Some(end), 1.days(), BarSize::Day,
    Some(WhatToShow::Trades), TradingHours::Regular,
).await?;

// v3.0 — pass the variant directly
client.historical_data(
    &contract, Some(end), 1.days(), BarSize::Day,
    WhatToShow::Trades, TradingHours::Regular,
).await?;
```

**Async `historical_schedule` split into two named methods:**

```rust,ignore
// v2.x — single method with Option<OffsetDateTime>
client.historical_schedule(&contract, None, 30.days()).await?;            // ending now
client.historical_schedule(&contract, Some(end), 30.days()).await?;       // anchored to date

// v3.0 — named methods, no magic None
client.historical_schedules_ending_now(&contract, 30.days()).await?;
client.historical_schedules(&contract, end, 30.days()).await?;
```

**Sync `interval_end` → `end_date`:**

The keyword-arg style stays the same; only the parameter name changed. Positional callers (the common case) are unaffected.

### 16. `ibapi::proto` is no longer public

The raw protobuf wire types and their encoders/decoders were never intended as a stable surface — they're a generated mirror of the upstream `.proto` files, and any upstream field rename would have been a silent breaking change for anyone who imported them. Consume the domain types (`Contract`, `Order`, `Execution`, …) directly. If you need a public conversion path, file an issue.

```rust,ignore
// v2.x — proto types reachable
use ibapi::proto::Contract;

// v3.0 — proto module is crate-private; use the domain type
use ibapi::contracts::Contract;
```

### 17. `ibapi::messages` is now opaque

The user-facing types from `ibapi::messages` — `Notice`, `NoticeCategory`, `IncomingMessages`, `OutgoingMessages`, and the notice-code-range constants (`WARNING_CODE_RANGE`, `SYSTEM_MESSAGE_CODES`, `ORDER_REJECTION_CODE_RANGE`, `ORDER_CANCELLED_CODE`, `HANDSHAKE_UNKNOWN_FRAME_CODE`, `HANDSHAKE_DECODE_FAILURE_CODE`) — are now re-exported from the crate root. `Notice` and `NoticeCategory` are also in the prelude. The wire-level types (`RequestMessage`, `ResponseMessage`, length-framing helpers, message-id index helpers) are crate-private in 3.0; downstream code never had a reason to reach them.

```rust,ignore
// v2.x
use ibapi::messages::{Notice, NoticeCategory, IncomingMessages};

// v3.0 — re-exports at crate root
use ibapi::{Notice, NoticeCategory, IncomingMessages};
// or simply
use ibapi::prelude::*;
```

### 18. `#[must_use]` on builders and subscription handles

Every fluent builder and subscription handle now carries `#[must_use]`. Forgetting the terminator (`.build()` / `.submit()` / `.subscribe()` / polling via `.next()` / `.next().await` / `.iter()`) used to be a silent no-op — the request never went out, the stream was dropped before reading. In 3.0 the same code emits an `unused_must_use` warning at the call site.

Affected types:

- **Subscription handles** — `Subscription<T>` (sync + async), `NoticeStream` (sync + async), `DisplayGroupSubscription` (sync + async), `TickSubscription` (sync + async). Dropping immediately cancels the request.
- **Contract builders** — `ContractBuilder` (field-minimal) and the typed entry points: `StockBuilder`, `OptionBuilder`, `FuturesBuilder`, `ContinuousFuturesBuilder`, `ForexBuilder`, `CryptoBuilder`, `SpreadBuilder`, `LegBuilder`. Terminate with `.build()` (or `.done()` for `LegBuilder`).
- **Order builders** — `OrderBuilder`, `BracketOrderBuilder`. Terminate with `.submit()` (canonical) or `.build()` (offline construction).
- **Market data builders** — `MarketDataBuilder`, `RealtimeBarsBuilder`. Terminate with `.subscribe()`.
- **Algo + condition builders** — 14 algo builders (`VwapBuilder`, `TwapBuilder`, `PctVolBuilder`, `ArrivalPriceBuilder`, `AdaptiveBuilder`, `ClosePriceBuilder`, `DarkIceBuilder`, `AccumulateDistributeBuilder`, `BalanceImpactRiskBuilder`, `MinimiseImpactBuilder`, `PctVolPriceBuilder`, `PctVolSizeBuilder`, `PctVolTimeBuilder`, `AccuDistrBuilder`) and 6 condition builders (`PriceConditionBuilder`, `TimeConditionBuilder`, `MarginConditionBuilder`, `ExecutionConditionBuilder`, `VolumeConditionBuilder`, `PercentChangeConditionBuilder`). Terminate with `.build()`.
- `ClientBuilder` already carried `#[must_use]` in earlier 3.0 work.

Not a hard break — code compiles unless you build with `-D warnings`. If you intentionally need to discard a handle (e.g. fire-and-forget cancel-order in a cleanup path), bind it explicitly:

```rust,ignore
let _ = client.cancel_order(order_id, "").await?;     // intentional drop
let _builder = order_builder;                          // keep alive without finalizing
```

### 19. `TagValue` moved out of `ibapi::orders`

`TagValue` is a generic key/value pair used across scanner filters, market-data options, order misc options, and combo routing — it never belonged in `orders`. In 3.0 it lives only at its canonical home `ibapi::contracts::TagValue`; the historical `ibapi::orders::TagValue` re-export is removed.

```rust,ignore
// v2.x
use ibapi::orders::TagValue;

// v3.0
use ibapi::contracts::TagValue;
```

No type changes — `TagValue` itself is unchanged. Only the import path moves.

### 20. `TickType` moved out of `ibapi::market_data::realtime`

`TickType` is the tick-discriminator enum (`Bid`, `Ask`, `Last`, `BidSize`, …) used in real-time tick payloads. In 3.0 it lives only at its canonical home `ibapi::contracts::tick_types::TickType`; the historical `ibapi::market_data::realtime::TickType` re-export is removed.

```rust,ignore
// v2.x
use ibapi::market_data::realtime::TickType;

// v3.0
use ibapi::contracts::tick_types::TickType;
```

No type changes — `TickType` itself is unchanged. Only the import path moves.

### 21. `contracts::builders::*` and `contracts::types::*` paths removed

`ibapi::contracts::builders` and `ibapi::contracts::types` were internal grouping submodules; their public items were already re-exported at `ibapi::contracts::*` via `pub use builders::*;` and `pub use types::*;`. Both submodules are now `pub(crate)` so the canonical short path is the only one. No types moved — all items remain reachable at `ibapi::contracts::*` (and via the prelude).

```rust,ignore
// v2.x
use ibapi::contracts::builders::{StockBuilder, OptionBuilder, FuturesBuilder};
use ibapi::contracts::types::{Symbol, Exchange, Currency};

// v3.0
use ibapi::contracts::{StockBuilder, OptionBuilder, FuturesBuilder};
use ibapi::contracts::{Symbol, Exchange, Currency};
```

### 22. `orders::builder::{OrderBuilder, BracketOrderBuilder, BracketOrderIds, OrderId}` paths removed

These four types were reachable at both `orders::*` (the canonical home, hoisted via `pub use builder::{...}`) and `orders::builder::*` (the duplicate, hoisted via `pub use order_builder::{...}` / `pub use types::{...}` inside `orders/builder/mod.rs`). The `orders::builder` duplicate is removed; the canonical `orders::*` path is unchanged.

```rust,ignore
// v2.x
use ibapi::orders::builder::{OrderBuilder, BracketOrderBuilder, BracketOrderIds, OrderId};

// v3.0
use ibapi::orders::{OrderBuilder, BracketOrderBuilder, BracketOrderIds, OrderId};
```

The low-level fluent layer (`orders::builder::price`, `orders::builder::time`, algo builders, etc.) is unchanged — only the four duplicate top-level builder/id paths move.

### 23. `Execution.side` typed as `ExecutionSide`

Was `String` in 2.x; in 3.0 it is `ExecutionSide`, a two-variant enum matching IBKR's documented wire vocabulary (C# `Execution.cs:83`): `"BOT"` → [`ExecutionSide::Bought`](https://docs.rs/ibapi/latest/ibapi/orders/enum.ExecutionSide.html), `"SLD"` → `ExecutionSide::Sold`. Short-sale fills emit `"SLD"` — the SSHORT designation lives on the originating [`Action`](https://docs.rs/ibapi/latest/ibapi/orders/enum.Action.html), not on the execution.

```rust,ignore
// v2.x — magic-string compare
if exec.side == "BOT" {
    handle_buy_fill();
}

// v3.0 — typed match
match exec.side {
    ExecutionSide::Bought => handle_buy_fill(),
    ExecutionSide::Sold   => handle_sell_fill(),
}
```

`ExecutionSide` implements `Display` (round-trips back to the wire string), `FromStr` (returns `Err(Error::Parse)` on unknown / empty inputs — the decoder fails loudly rather than silently defaulting), and is exposed via `ibapi::prelude::*`. Existing `println!("{}", exec.side)` callsites continue to print `"BOT"` / `"SLD"` unchanged thanks to `Display`.

### 24. `historical_schedules` collapses to a builder

In 2.x there were two methods:

```rust,ignore
// v2.x — anchored to a specific end date
let schedule = client.historical_schedules(&contract, end_date, 30.days())?;

// v2.x — anchored at current time
let schedule = client.historical_schedules_ending_now(&contract, 30.days())?;
```

In 3.0 both collapse into one [`HistoricalScheduleBuilder`](https://docs.rs/ibapi/latest/ibapi/market_data/historical/struct.HistoricalScheduleBuilder.html). Default anchors at the current time; call `.ending(end_date)` to anchor at a specific date:

```rust,ignore
// v3.0 — anchored at current time (default)
let schedule = client.historical_schedules(&contract, 30.days()).fetch()?;

// v3.0 — anchored to a specific end date
let schedule = client
    .historical_schedules(&contract, 30.days())
    .ending(end_date)
    .fetch()?;
```

`historical_schedules_ending_now` is removed.

### 25. `historical_ticks_*` trio collapses to a builder

In 2.x there were three methods, one per tick type, with near-identical 5–6 arg signatures:

```rust,ignore
// v2.x
let trades = client.historical_ticks_trade(&contract, Some(start), None, 100, TradingHours::Regular)?;
let mids   = client.historical_ticks_mid_point(&contract, Some(start), None, 100, TradingHours::Regular)?;
let quotes = client.historical_ticks_bid_ask(&contract, Some(start), None, 100, TradingHours::Regular, false)?;
```

In 3.0 the three collapse into one [`HistoricalTicksBuilder`](https://docs.rs/ibapi/latest/ibapi/market_data/historical/struct.HistoricalTicksBuilder.html). The terminal method selects the tick type:

```rust,ignore
// v3.0
let trades = client.historical_ticks(&contract, 100).starting(start).trade()?;
let mids   = client.historical_ticks(&contract, 100).starting(start).mid_point()?;
let quotes = client.historical_ticks(&contract, 100).starting(start).bid_ask(IgnoreSize::No)?;
```

The `ignore_size: bool` parameter (previously only valid for the bid/ask variant) is now an [`IgnoreSize`](https://docs.rs/ibapi/latest/ibapi/market_data/historical/enum.IgnoreSize.html) enum (`Yes` / `No`) and lives only on the `.bid_ask(...)` terminal where IBKR honors it. Other setters: `.ending(end)` to anchor at an end date, `.trading_hours(TradingHours)` to override the default `Regular`.

### 26. `historical_data` + `historical_data_streaming` collapse to a builder

In 2.x there were two methods, both with 6 args:

```rust,ignore
// v2.x
let bars = client.historical_data(
    &contract, Some(end_date), 7.days(),
    BarSize::Hour, WhatToShow::Trades, TradingHours::Regular,
)?;

let sub = client.historical_data_streaming(
    &contract, 1.days(), BarSize::Min15, WhatToShow::Trades,
    TradingHours::Regular, true, // keep_up_to_date
)?;
```

In 3.0 both collapse into one [`HistoricalDataBuilder`](https://docs.rs/ibapi/latest/ibapi/market_data/historical/struct.HistoricalDataBuilder.html) with terminal-typed output:

```rust,ignore
// v3.0 — one-shot
let bars = client
    .historical_data(&contract, BarSize::Hour)
    .duration(7.days())
    .ending(end_date)
    .fetch()?;

// v3.0 — streaming (keep_up_to_date always true)
let sub = client
    .historical_data(&contract, BarSize::Min15)
    .duration(1.days())
    .stream()?;
```

Two date-spec styles are supported and mutually exclusive at the terminal:

- **IBKR-native**: `.duration(D)` (defaults `end_date = None` → now) with optional `.ending(end)` to anchor a specific end date.
- **Range** (convenience): `.between(start, end)` — computes duration internally and sets `end_date = end`.

Mixing the two (`.between` together with `.duration` or `.ending`) returns `Err(Error::InvalidArgument)` from the terminal. `.stream()` rejects builders that called `.ending(...)` or `.between(...)` — IBKR requires `end_date = None` for `keep_up_to_date = true`.

Other setters: `.what_to_show(WhatToShow)` (default `Trades`), `.trading_hours(TradingHours)` (default `Regular`).

The `historical_data_streaming` method's `keep_up_to_date: bool` parameter is gone — `.stream()` always sets it to `true`. The `keep_up_to_date = false` case wasn't a useful public-API combination (the same wire shape is reachable via `.fetch()` with a different return type).

### 27. `IgnoreSize` moved from `historical::` to `market_data::`

`IgnoreSize` (introduced in PR #613 as part of the historical-ticks builder) was scoped to `ibapi::market_data::historical::IgnoreSize`. The same wire flag applies to realtime tick-by-tick subscriptions, so the enum was lifted to `ibapi::market_data::IgnoreSize` to share between both submodules.

```rust,ignore
// v3 pre-#XXX
use ibapi::market_data::historical::IgnoreSize;

// v3 ≥ #XXX
use ibapi::market_data::IgnoreSize;
```

No type changes — `IgnoreSize` itself is unchanged (still `Yes` / `No`). The prelude entry is unchanged; users who import via `ibapi::prelude::*;` see no difference.

### 28. `tick_by_tick_*` quartet collapses to a builder

In 2.x there were four methods, one per tick type, all with the same 3-arg signature:

```rust,ignore
// v2.x
let trades = client.tick_by_tick_all_last(&contract, 10, false)?;
let lasts  = client.tick_by_tick_last(&contract, 10, false)?;
let quotes = client.tick_by_tick_bid_ask(&contract, 10, false)?;
let mids   = client.tick_by_tick_midpoint(&contract, 10, false)?;
```

In 3.0 the four collapse into one [`TickByTickBuilder`](https://docs.rs/ibapi/latest/ibapi/market_data/realtime/struct.TickByTickBuilder.html). The terminal method selects the tick stream:

```rust,ignore
// v3.0
let trades = client.tick_by_tick(&contract, 10).all_last()?;
let lasts  = client.tick_by_tick(&contract, 10).last()?;
let quotes = client.tick_by_tick(&contract, 10).bid_ask(IgnoreSize::No)?;
let mids   = client.tick_by_tick(&contract, 10).mid_point()?;
```

The `ignore_size: bool` parameter (only meaningful for the bid/ask variant — IBKR ignores it on the other three) is now an [`IgnoreSize`](https://docs.rs/ibapi/latest/ibapi/market_data/enum.IgnoreSize.html) enum (`Yes` / `No`) and lives only on the `.bid_ask(...)` terminal where IBKR honors it.

### 29. `market_depth` becomes a builder; `bool` → `SmartDepth`

The 3-arg `market_depth(&contract, num_rows, is_smart_depth)` collapses into a [`MarketDepthBuilder`](https://docs.rs/ibapi/latest/ibapi/market_data/realtime/struct.MarketDepthBuilder.html) with a `.smart_depth(SmartDepth)` setter and a `.subscribe()` terminal. The stringly-typed `is_smart_depth: bool` is now a typed [`SmartDepth`](https://docs.rs/ibapi/latest/ibapi/market_data/enum.SmartDepth.html) enum (`Yes` / `No`, default `No`), mirroring [`IgnoreSize`](https://docs.rs/ibapi/latest/ibapi/market_data/enum.IgnoreSize.html).

```rust,ignore
// v2.x
let book = client.market_depth(&contract, 5, true)?;
```

```rust,ignore
// v3.0
use ibapi::market_data::SmartDepth;

let book = client.market_depth(&contract, 5)
    .smart_depth(SmartDepth::Yes)
    .subscribe()?;
```

`SmartDepth::No` is the default — callers that previously passed `false` can omit `.smart_depth(...)` entirely.

### 30. `Bar.date` typed as `BarTimestamp` (historical bars)

Historical `Bar.date` was `OffsetDateTime`. Daily bars carried a `YYYYMMDD` wire value that was coerced to midnight UTC — semantically wrong (a trading day is not a point in time).

`Bar.date` is now `BarTimestamp`, an enum that preserves the wire distinction:

```rust,ignore
pub enum BarTimestamp {
    Date(time::Date),           // daily / weekly / monthly bars
    DateTime(time::OffsetDateTime), // intraday bars
}
```

**Before (v2.x / v3 pre-#627):**

```rust,ignore
println!("{:02}:{:02}", bar.date.hour(), bar.date.minute());
```

**After (v3 ≥ #627):**

```rust,ignore
use ibapi::market_data::historical::BarTimestamp;

match &bar.date {
    BarTimestamp::Date(d) => println!("{d}"),
    BarTimestamp::DateTime(dt) => println!("{:02}:{:02}", dt.hour(), dt.minute()),
}
```

`BarTimestamp` implements `Display`, `FromStr`, `From<Date>`, and `From<OffsetDateTime>`. The realtime `Bar` (in `market_data::realtime`) is unchanged — it always carries `OffsetDateTime`.

## Before / after: common subscription patterns

### Order construction

3.0 picks `client.order(&contract).buy(qty).<type>().submit()` as the canonical fluent
path. `submit()` allocates the order id internally (no manual `next_order_id()` step) and
uses fire-and-forget delivery; status flows through
[`Client::order_update_stream`](https://docs.rs/ibapi/latest/ibapi/client/struct.Client.html#method.order_update_stream).
The `order_builder::*` free functions still exist and are unchanged — they are now
documented as the *advanced / client-less* layer (BYO order id, offline construction,
hand-composed multi-leg orders). For BYO-id flows with the fluent builder,
`OrderBuilder::build_order()` returns a bare `Order` you can submit yourself.

```rust,ignore
// v2.x — manual id + free-fn order construction + place_order
let order_id = client.next_order_id();
let order = order_builder::limit_order(Action::Buy, 100.0, 150.0);
client.place_order(order_id, &contract, &order)?;
```

```rust,ignore
// v3.0 (sync) — fluent: side implies action; submit() allocates the id
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.0)
    .submit()?;
```

```rust,ignore
// v3.0 (async)
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.0)
    .submit().await?;
```

```rust,ignore
// v3.0 — bracket order: entry + take-profit + stop-loss in one chain
let bracket_ids = client.order(&contract)
    .buy(100)
    .bracket()
    .entry_limit(150.00)
    .take_profit(160.00)
    .stop_loss(145.00)
    .submit_all()?;
```

```rust,ignore
// v3.0 — BYO order id (advanced): use OrderBuilder::build_order() and submit yourself
let order = client.order(&contract).buy(100).limit(150.0).build_order()?;
let order_id = my_external_allocator.next();
client.place_order(order_id, &contract, &order)?;
```

`client.next_order_id()` is still public for the BYO-id path; it just isn't shown in the
canonical happy-path examples anymore.

The fluent path covers all four `Action` variants. `.buy(qty)` and `.sell(qty)` are the
common cases; `.sell_short(qty)` (`SSHORT` — institutional Long/Short account segments)
and `.sell_long(qty)` (`SLONG` — selling not-yet-delivered long position) cover the
specialized accounts. Callers that dispatch on a runtime `Action` value can match
exhaustively without a `_ => unreachable!()` arm.

### Market data

```rust,ignore
// v2.x (sync)
let sub = client.market_data(&contract).generic_ticks(&["233"]).subscribe()?;
for tick in sub {
    println!("tick: {tick:?}");
}
```

```rust,ignore
// v3.0 (sync) — data-only; explicit Err arm so terminal errors aren't dropped
use ibapi::prelude::*;
let sub = client.market_data(&contract).generic_ticks(&["233"]).subscribe()?;
for item in sub.iter_data() {
    match item {
        Ok(tick) => println!("tick: {tick:?}"),
        Err(e)   => { eprintln!("error: {e}"); break; }
    }
}
```

### Order placement

`Subscription<PlaceOrder>` already had multiple variants in 2.x; in 3.0 the wrapper widens to `SubscriptionItem<PlaceOrder>` so it can also surface unsolicited per-order notices alongside `OrderStatus`/`OpenOrder`/etc.

```rust,ignore
// v2.x
let events = client.place_order(order_id, &contract, &order)?;
for event in events {
    match event {
        PlaceOrder::OrderStatus(s)      => println!("status: {s:?}"),
        PlaceOrder::OpenOrder(o)        => println!("open: {o:?}"),
        PlaceOrder::ExecutionData(e)    => println!("exec: {e:?}"),
        PlaceOrder::CommissionReport(c) => println!("commission: {c:?}"),
        PlaceOrder::Message(m)          => println!("message: {m:?}"),
    }
}
```

```rust,ignore
// v3.0 — `PlaceOrder::Message` is gone (notices route via SubscriptionItem::Notice)
use ibapi::prelude::*;
let events = client.place_order(order_id, &contract, &order)?;
for event in events.iter_data() {
    match event? {
        PlaceOrder::OrderStatus(s)      => println!("status: {s:?}"),
        PlaceOrder::OpenOrder(o)        => println!("open: {o:?}"),
        PlaceOrder::ExecutionData(e)    => println!("exec: {e:?}"),
        PlaceOrder::CommissionReport(c) => println!("commission: {c:?}"),
    }
}
```

If you want to surface per-order TWS warnings (e.g. quote-throttling 2100 codes scoped to the order), iterate the full envelope:

```rust,ignore
// v3.0 — observe both order events and per-order notices
for item in &events {
    match item {
        Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(s))) => println!("status: {s:?}"),
        Ok(SubscriptionItem::Data(_other))                     => {}
        Ok(SubscriptionItem::Notice(note))                     => eprintln!("order notice: {note}"),
        Err(e)                                                 => { eprintln!("error: {e}"); break; }
    }
}
```

### Account summary

```rust,ignore
// v2.x
let sub = client.account_summary(&AccountGroup::All, &["NetLiquidation"])?;
for row in sub {
    println!("row: {row:?}");
}
```

```rust,ignore
// v3.0 — data only; explicit Err arm so terminal errors aren't dropped
use ibapi::prelude::*;
let sub = client.account_summary(&AccountGroup::All, &["NetLiquidation"])?;
for item in sub.iter_data() {
    match item {
        Ok(row) => println!("row: {row:?}"),
        Err(e)  => { eprintln!("error: {e}"); break; }
    }
}
```

## New: globally routed notices

Notices that arrive without a `request_id` (connectivity, farm status, generic warnings) used to be log-only. Subscribe to them programmatically via `Client::notice_stream()`:

```rust,ignore
use ibapi::client::blocking::Client;

let client = Client::connect("127.0.0.1:4002", 100)?;
let stream = client.notice_stream()?;
for notice in stream.iter() {
    if notice.is_system_message() {
        println!("connectivity: {notice}");
    } else if notice.is_warning() {
        println!("warning: {notice}");
    } else {
        eprintln!("error: {notice}");
    }
}
```

The async version is symmetric — `client.notice_stream()` returns a handle whose `next().await` yields `Notice` values (and a `stream()` adapter for `futures::StreamExt`).

### Connect entry points unified on `Client::builder()`

v3.0 collapses three connect paths (`connect`, `connect_with_callback`, `connect_with_options`) into a fluent builder. `Client::connect(addr, id)` stays as a no-options shortcut; everything else moves to the builder.

**Removed in this release** (no compat shim):

- `Client::connect_with_callback`
- `Client::connect_with_options`
- `ConnectionOptions` (struct + builder methods)
- `StartupMessageCallback` (type alias)
- `StartupNoticeCallback` (type alias)

Before:

```rust,ignore
use ibapi::client::blocking::Client;
use ibapi::ConnectionOptions;

let options = ConnectionOptions::default()
    .tcp_no_delay(true)
    .startup_notice_callback(|notice| println!("startup: {notice}"));

let client = Client::connect_with_options("127.0.0.1:4002", 100, options)?;
```

After:

```rust,ignore
use ibapi::client::blocking::Client;

let (client, notices) = Client::builder()
    .address("127.0.0.1:4002")
    .client_id(100)
    .tcp_no_delay(true)
    .connect_with_notice_stream()?;

for n in notices.iter() {
    println!("startup: {n}");
}
```

The pre-bound `NoticeStream` from `connect_with_notice_stream()` captures handshake-time notices (2104/2106/2158 farm-status, 1100/1101/1102 connectivity) AND every unrouted notice for the lifetime of the connection. It survives auto-reconnects — the broadcaster lives on `Connection`, not on the bus.

If you don't need handshake notices, use `.connect()` instead — it returns just the `Client`.

The startup-message callback (typed `OpenOrder` / `OrderStatus` / account updates emitted during the handshake) is now a builder configurator:

```rust,ignore
use ibapi::{Client, StartupMessage};

let client = Client::builder()
    .address("127.0.0.1:4002")
    .client_id(100)
    .startup_callback(|msg| if let StartupMessage::OpenOrder(o) = msg {
        println!("startup open order: {}", o.order_id);
    })
    .connect()
    .await?;
```

The async builder lives at `ibapi::Client::builder()` (top-level alias under default features); the sync builder at `ibapi::client::blocking::Client::builder()`.

### `StartupMessage` gains typed `Execution` / `CommissionReport` / `CompletedOrder` variants

If you matched on `StartupMessage::Other(rm)` and called `rm.message_type()` to dispatch on `ExecutionData` / `CommissionsReport` / `CompletedOrder` (typically when connected as the Master Client ID, where TWS replays open-order + commission-report history at handshake), switch to the new typed variants — the payload is pre-decoded:

```rust,ignore
// 2.x / earlier 3.0
match msg {
    StartupMessage::Other(rm) if rm.message_type() == IncomingMessages::ExecutionData => {
        // ... decode rm yourself ...
    }
    StartupMessage::Other(rm) if rm.message_type() == IncomingMessages::CommissionsReport => { /* ... */ }
    StartupMessage::Other(rm) if rm.message_type() == IncomingMessages::CompletedOrder => { /* ... */ }
    // ...
}

// 3.x current
match msg {
    StartupMessage::Execution(execution) => { /* typed ExecutionData payload */ }
    StartupMessage::CommissionReport(report) => { /* typed CommissionReport payload */ }
    StartupMessage::CompletedOrder(order) => { /* typed OrderData; order_id is -1 */ }
    StartupMessage::ExecutionDataEnd => { /* end-of-executions marker */ }
    StartupMessage::CompletedOrdersEnd => { /* end-of-completed-orders marker */ }
    // ...
}
```

`StartupMessage` is now `#[non_exhaustive]`. Add a `_` arm to any exhaustive match if you weren't writing one already — future variants will land here as additional handshake-time message kinds are typed.

### `StartupMessage::Other` removed; `ResponseMessage` is crate-private

The `Other(ResponseMessage)` variant is gone, and `ibapi::ResponseMessage` is no longer reachable from outside the crate.

Unsolicited handshake-time messages that aren't one of the typed kinds (or whose typed decoder fails) are now routed to `Client::notice_stream()` with synthesized codes:

- `HANDSHAKE_UNKNOWN_FRAME_CODE` (`-3`) — TWS sent a frame kind that has no typed `StartupMessage` variant.
- `HANDSHAKE_DECODE_FAILURE_CODE` (`-4`) — a typed decoder failed on a known kind (e.g. truncated wire bytes).

Use `Notice::is_handshake_synthetic()` to detect them:

```rust,ignore
use ibapi::{Client, Notice};

let (client, notices) = Client::builder()
    .address("127.0.0.1:4002")
    .client_id(0)
    .connect_with_notice_stream()
    .await?;

for n in notices.iter() {
    if n.is_handshake_synthetic() {
        eprintln!("handshake observability: code={} msg={}", n.code, n);
    }
}
```

If you previously matched on `StartupMessage::Other(_)` to log "unexpected handshake frame," subscribe to the notice stream instead.

The `Error::UnexpectedResponse` variant changed from `UnexpectedResponse(ResponseMessage)` to `UnexpectedResponse(String)`. The string carries the `Debug` repr of the offending wire envelope for diagnostic logging; the structured payload is no longer exposed. `matches!(err, Error::UnexpectedResponse(_))` continues to work unchanged.

New in 3.0: `Error::ConnectionRejected(String)` is fired by `Client::connect` when TWS/Gateway accepts the TCP socket and then closes before completing the handshake — typically a host allow-list mismatch on the gateway. Previously this surfaced as `Error::Simple` with a `"The server may be rejecting connections from this host: ..."` prefix; the typed variant lets callers distinguish allow-list failure from generic connection failure without string matching.

```rust
match Client::connect("127.0.0.1:4002", 100) {
    Ok(client) => { /* ... */ }
    Err(Error::ConnectionRejected(msg)) => {
        eprintln!("gateway rejected connection: {msg} — check 'Trusted IPs' in TWS/Gateway settings");
    }
    Err(err) => eprintln!("connect failed: {err}"),
}
```

### `Error::Message` → `Error::Notice(Notice)`

TWS-emitted error frames now arrive as `Error::Notice(Notice)` instead of `Error::Message(i32, String)` on `Result<_, Error>` returns. The new variant carries the full typed [`Notice`] (code, message, `error_time`, `advanced_order_reject_json`) and exposes the same classification API as the streaming side:

```rust
// before
match client.contract_details(&contract) {
    Ok(details) => { /* ... */ }
    Err(Error::Message(code, msg)) if (200..=399).contains(&code) => {
        eprintln!("rejection [{code}]: {msg}");
    }
    Err(Error::Message(code, msg)) => eprintln!("TWS error [{code}]: {msg}"),
    Err(err) => eprintln!("transport: {err}"),
}

// after
match client.contract_details(&contract) {
    Ok(details) => { /* ... */ }
    Err(Error::Notice(n)) if n.is_order_rejection() => eprintln!("rejection: {n}"),
    Err(Error::Notice(n)) => match n.category() {
        NoticeCategory::Warning       => eprintln!("warn: {n}"),
        NoticeCategory::SystemMessage => eprintln!("system: {n}"),
        _                             => eprintln!("error: {n}"),
    },
    Err(err) => eprintln!("transport: {err}"),
}
```

This makes `Result<_, Error>` returns symmetric with `Subscription<T>` items, which already yield `SubscriptionItem::Notice(Notice)` for the same wire frame. As a bonus, the projection now preserves `error_time` and `advanced_order_reject_json` — the old `Error::Message` shape dropped both.

Distinct from `Error::ConnectionRejected` (handshake-time refusal, above) and the transport variants (`Error::Io`, `Error::ConnectionReset`).

## Quick migration checklist

1. Replace `for x in &subscription` with `for item in subscription.iter_data() { match item { Ok(x) => ..., Err(e) => ... } }` (sync) or the equivalent on `subscription.data_stream()` / `subscription.next_data()` (async). `iter_data().flatten()` is shorter but silently drops terminal errors — use it only when that's intentional.
2. Use `for item in &subscription { match item { Ok(SubscriptionItem::Data(_))..., Ok(SubscriptionItem::Notice(_))..., Err(_)... } }` when you want full visibility.
3. Replace `subscription.error()` / `subscription.clear_error()` with pattern-matching on the `Err` arm of `next()`.
4. Drop any `match` arms for `PlaceOrder::Message` / `OrderUpdate::Message` / similar per-T notice variants — those arms are unreachable and the variants are gone. Route per-order notices via `SubscriptionItem::Notice` and global notices via `Client::notice_stream()`.
5. Replace string compares against `OrderStatus.status` / `OrderState.status` (`== "Filled"`, `.as_str() == "Cancelled"`, etc.) with `OrderStatusKind` variants or the `is_active()` / `is_terminal()` helpers.
6. Replace `Client::connect_with_callback` / `Client::connect_with_options` / any `ConnectionOptions::default()...` with the corresponding `Client::builder()` chain. Use `connect_with_notice_stream()` if you previously installed `startup_notice_callback`.
7. (Optional) Adopt `Client::notice_stream()` for runtime-only unrouted notice observability.
8. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and your test suite for each feature flag you support.

## Need help?

- Examples: `examples/async` and `examples/sync`
- README: [Handling notifications](../README.md#handling-notifications)
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
