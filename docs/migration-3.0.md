# Migration Guide: 2.x to 3.0

Version 3.0 is a breaking release. This guide walks through the changes required to upgrade from `ibapi` 2.x to 3.0. For 1.x → 2.x, see [`MIGRATION.md`](../MIGRATION.md).

## Highlights

- `Subscription<T>::next()` now returns `Option<Result<SubscriptionItem<T>, Error>>`. The new `SubscriptionItem<T>` enum has two arms: `Data(T)` for decoded payloads and `Notice(Notice)` for non-fatal IB notices that share the subscription's `request_id`.
- `Subscription::error()` and `Subscription::clear_error()` are removed. Terminal errors surface as `Some(Err(_))` on the next call to `next()`; subsequent calls return `None`.
- New `Client::notice_stream()` exposes globally routed IB notices (connectivity codes 1100/1101/1102, farm-status 2104/2105/2106/2107/2108, etc.) that are not tied to any subscription.
- `ConnectionOptions::startup_callback` and `startup_notice_callback` provide typed access to messages and notices emitted during the connection handshake.
- The text wire protocol is gone; v3.0 is protobuf-only and requires a TWS/IB Gateway server version that supports it.

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
// v3.0 — data-only iteration; terminal errors visible, notices logged
for bar in subscription.iter_data().flatten() {
    println!("bar: {bar:?}");
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

## Before / after: common subscription patterns

### Market data

```rust,ignore
// v2.x (sync)
let sub = client.market_data(&contract).generic_ticks(&["233"]).subscribe()?;
for tick in sub {
    println!("tick: {tick:?}");
}
```

```rust,ignore
// v3.0 (sync) — data-only
use ibapi::prelude::*;
let sub = client.market_data(&contract).generic_ticks(&["233"]).subscribe()?;
for tick in sub.iter_data().flatten() {
    println!("tick: {tick:?}");
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
// v3.0 — data-only is still ergonomic
use ibapi::prelude::*;
let events = client.place_order(order_id, &contract, &order)?;
for event in events.iter_data() {
    match event? {
        PlaceOrder::OrderStatus(s)      => println!("status: {s:?}"),
        PlaceOrder::OpenOrder(o)        => println!("open: {o:?}"),
        PlaceOrder::ExecutionData(e)    => println!("exec: {e:?}"),
        PlaceOrder::CommissionReport(c) => println!("commission: {c:?}"),
        PlaceOrder::Message(m)          => println!("message: {m:?}"),
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
// v3.0 — data only
use ibapi::prelude::*;
let sub = client.account_summary(&AccountGroup::All, &["NetLiquidation"])?;
for row in sub.iter_data().flatten() {
    println!("row: {row:?}");
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

### Handshake-time notices need a startup callback

The 2104/2106/2158 farm-status notices typically arrive *before* `Client::connect` returns, so a `notice_stream` registered after connect won't see them. Use `ConnectionOptions::startup_notice_callback` for handshake-time observability:

```rust,ignore
use ibapi::client::blocking::Client;
use ibapi::ConnectionOptions;

let options = ConnectionOptions::default()
    .startup_notice_callback(|notice| {
        if notice.is_system_message() {
            println!("startup connectivity: {notice}");
        } else {
            println!("startup notice: {notice}");
        }
    });

let client = Client::connect_with_options("127.0.0.1:4002", 100, options)?;
```

`startup_notice_callback` fires for every handshake — initial connect *and* auto-reconnect.

## Quick migration checklist

1. Replace `for x in &subscription` with `for x in subscription.iter_data().flatten()` (sync) or `subscription.next_data()` / `subscription.data_stream()` (async) when you only want data.
2. Use `for item in &subscription { match item { Ok(SubscriptionItem::Data(_))..., Ok(SubscriptionItem::Notice(_))..., Err(_)... } }` when you want full visibility.
3. Replace `subscription.error()` / `subscription.clear_error()` with pattern-matching on the `Err` arm of `next()`.
4. (Optional) Adopt `Client::notice_stream()` for runtime-only unrouted notice observability.
5. (Optional) Adopt `ConnectionOptions::startup_notice_callback` and `startup_callback` for handshake-time observability.
6. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and your test suite for each feature flag you support.

## Need help?

- Examples: `examples/async` and `examples/sync`
- README: [Handling notifications](../README.md#handling-notifications)
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
