# Migration Guide: 3.x to 4.0

Version 4.0 is a breaking release. This guide walks through the changes required to upgrade from `ibapi` 3.x to 4.0. For 2.x → 3.x, see [`migration-3.0.md`](migration-3.0.md); for 1.x → 2.x, see [`MIGRATION.md`](../MIGRATION.md).

## Highlights

- Market-data sizes that IBKR models as decimals are now `Option<f64>` instead of `i32` / `f64` — fractional sizes (crypto, fractional shares) no longer truncate to `0`, and "TWS sent no value" is distinguishable from a real zero.
- `Liquidity` preserves unrecognized execution liquidity codes as `Liquidity::Unknown(i32)` instead of collapsing them to `Liquidity::None`. Exhaustive matches need a new arm.
- `Client::wsh_event_data_by_contract` / `wsh_event_data_by_filter` return builders instead of taking positional `Option` arguments.
- `Client::check_server_version` is crate-private; compare against `Client::server_version()` directly.
- `Notice` gains a `request_id` field — the originating request or order id, `None` for request-less notices.
- Transport hardening (behavioral, not API-breaking): frame-length validation with automatic reconnect, wrong-wire-format detection, unified retry-on-reset across one-shot requests, a configurable reconnect attempt limit (`ClientBuilder::max_reconnect_attempts` / `reconnect_forever`), and byte-level stream capture via `IBAPI_RAW_CAPTURE_DIR`.

## Breaking changes

### 1. Market-data sizes are `Option<f64>`

IBKR models market-data sizes as decimals and ships them as strings on the wire. Historical tick and histogram sizes were typed `i32`, so a fractional wire value such as `"0.5"` failed integer parsing and silently decoded as `0` — real data loss on crypto and fractional-share feeds. Those fields, plus the `ContractDetails` size rules, are now `Option<f64>`.

| Type | Field | 3.x | 4.0 |
|---|---|---|---|
| `TickMidpoint` | `size` | `i32` | `Option<f64>` |
| `TickLast` | `size` | `i32` | `Option<f64>` |
| `TickBidAsk` | `size_bid`, `size_ask` | `i32` | `Option<f64>` |
| `HistogramEntry` | `size` | `i32` | `Option<f64>` |
| `ContractDetails` | `min_size`, `size_increment`, `suggested_size_increment` | `f64` | `Option<f64>` |

`None` means TWS sent no value — the field was absent, empty, or carried one of TWS's "unset" sentinels (`2147483647`, `9223372036854775807`, `-9223372036854775808`, `1.7976931348623157E308`). `Some(0.0)` is a real zero. A malformed size is no longer swallowed: it surfaces as `Error::Parse` and fails the request or subscription.

**The `Ord` gotcha.** `f64` does not implement `Ord`, so `max_by_key` / `min_by_key` on a size no longer compiles. This is the most likely break in existing code:

```rust,ignore
// 3.x — compiled against i32
let mode = histogram.iter().max_by_key(|e| e.size);

// 4.0 — pick out the present sizes, then compare with total_cmp
let mode = histogram
    .iter()
    .filter_map(|e| e.size.map(|s| (e, s)))
    .max_by(|a, b| a.1.total_cmp(&b.1));
```

**Accumulating.** Treat "unset" as contributing nothing. Use `unwrap_or(0.0)` when you are folding one value at a time into a running total, and `filter_map` when you are reducing a collection — the latter also matters for `min`/`max`, where a substituted `0.0` would skew the result:

```rust,ignore
// 3.x
let mut total_volume = 0;
total_volume += tick.size;

// 4.0
let mut total_volume = 0.0;
total_volume += tick.size.unwrap_or(0.0);

// Or, over a collection:
let total: f64 = histogram.iter().filter_map(|e| e.size).sum();
```

**Displaying.** `Option<f64>` is not `Display`, so `{}` no longer works. Prefer surfacing the missing case over hiding it behind a zero:

```rust,ignore
// 3.x
println!("Size: {}", tick.size);

// 4.0
fn fmt_size(size: Option<f64>) -> String {
    size.map_or_else(|| "n/a".to_string(), |s| format!("{s:.0}"))
}
println!("Size: {}", fmt_size(tick.size));
```

**Serialized shape changes too.** These types derive `Serialize`/`Deserialize` (and `utoipa::ToSchema` under the `utoipa` feature), so the JSON changes in two ways beyond the compile errors: a present size now serializes as `100.0` rather than `100`, and an absent one as `null` rather than `0`. Under `utoipa` the generated schema goes from `integer` to a nullable `number`. If you publish an OpenAPI contract or have strict JSON consumers downstream, that is a breaking change with no compile-time signal.

`Option<f64>` is an intermediate step. A dedicated decimal quantity type is planned, so sizes will eventually round-trip the wire's decimal representation exactly rather than through binary floating point.

### 2. `Liquidity` gains `Unknown(i32)`

`Execution.last_liquidity` used to collapse any liquidity code outside the documented 0–3 range to `Liquidity::None` — indistinguishable from a genuine "no liquidity information". Unrecognized codes are now preserved:

```rust,ignore
// 3.x — exhaustive match compiled
match execution.last_liquidity {
    Liquidity::None => {}
    Liquidity::AddedLiquidity => {}
    Liquidity::RemovedLiquidity => {}
    Liquidity::LiquidityRoutedOut => {}
}

// 4.0 — add an arm for the new variant
match execution.last_liquidity {
    Liquidity::None => {}
    Liquidity::AddedLiquidity => {}
    Liquidity::RemovedLiquidity => {}
    Liquidity::LiquidityRoutedOut => {}
    Liquidity::Unknown(code) => log::warn!("unrecognized liquidity code {code}"),
}
```

The documented codes 0–3 decode exactly as before. No official client or `Execution.proto` defines a code outside that range today, so `Unknown` is forward compatibility — a code IBKR adds later surfaces as `Unknown(code)` you can log, store, or reject, instead of silently masquerading as `None`. `Liquidity` is deliberately exhaustive (no `#[non_exhaustive]`), so the compiler points at every match that needs the new arm.

### 3. WSH event data goes through builders

`wsh_event_data_by_contract` took one required argument and four `Option`s; `wsh_event_data_by_filter` took one and two. Every call site in this repository — both examples and both integration tests — passed `None` for all of them. They now return builders, with one setter per optional value:

```rust,ignore
// 3.x
let events = client.wsh_event_data_by_contract(contract_id, None, None, None, None)?;
let events = client.wsh_event_data_by_contract(
    contract_id,
    Some(date!(2024 - 01 - 01)),
    Some(date!(2024 - 03 - 31)),
    Some(50),
    Some(auto_fill),
)?;
let subscription = client.wsh_event_data_by_filter(filter, None, None)?;

// 4.0
let events = client.wsh_event_data_by_contract(contract_id).fetch()?;
let events = client
    .wsh_event_data_by_contract(contract_id)
    .starting(date!(2024 - 01 - 01))
    .ending(date!(2024 - 03 - 31))
    .limit(50)
    .auto_fill(auto_fill)
    .fetch()?;
let subscription = client.wsh_event_data_by_filter(filter).subscribe()?;
```

The terminals differ because the requests do: `.fetch()` returns a single `WshEventData`, `.subscribe()` returns a `Subscription<WshEventData>`. Async is identical with `.await` on the terminal.

Each setter carries its own server-version requirement — `.starting()` / `.ending()` / `.limit()` need `WSH_EVENT_DATA_FILTERS_DATE`, `.auto_fill()` needs `WSH_EVENT_DATA_FILTERS` — so a bare request still works against a gateway that supports none of them. That was true before too; the builder just makes it visible which argument costs which version.

### 4. `Client::check_server_version` is crate-private

The async `Client` exposed `check_server_version(required_version, feature)` as `pub` while the blocking `Client` kept the same method `pub(crate)`. That was drift, not design — the method is the internal guard each version-gated API calls before encoding a request, and nothing in `examples/` or the integration crates ever called it. Both are now `pub(crate)`.

If you were calling it to branch on server support, compare against `Client::server_version()` directly:

```rust,ignore
// 3.x — async only
client.check_server_version(server_versions::SIZE_RULES, "size rules")?;

// 4.0 — the constants are public; the guard is not
if client.server_version() < server_versions::SIZE_RULES {
    // fall back
}
```

The version-gated methods still perform this check themselves and return `Error::ServerVersion` when the gateway is too old, so an explicit pre-check is only needed when you want to branch instead of erroring.

### 5. `Notice` gains `request_id`

`Notice` now carries the originating request or order id — `None` for request-less notices. `Notice` is not `#[non_exhaustive]`, so struct literals must add the field:

```rust,ignore
// 3.x
let notice = Notice {
    code: 2104,
    message: "Market data farm connection is OK".into(),
    error_time: None,
    advanced_order_reject_json: String::new(),
};

// 4.0
let notice = Notice {
    request_id: None,
    code: 2104,
    message: "Market data farm connection is OK".into(),
    error_time: None,
    advanced_order_reject_json: String::new(),
};
```

Consumers are unaffected at compile time but gain information: a notice delivered to a subscription or the order-update stream now names the request or order it belongs to. The serde shape is backward compatible — `request_id` is `#[serde(default, skip_serializing_if = "Option::is_none")]`, so 3.x JSON still deserializes and the field only appears in output when present.

### 6. `DATA_ADVISORY_CODES` widens to `[i32; 4]`

The advisory list grows from `[10089, 10167]` to `[2188, 10089, 10090, 10167]` — see the notice-classification changes under [Behavioral changes](#behavioral-changes). This is breaking only for code binding the const with an explicit array type:

```rust,ignore
// 3.x
let advisories: [i32; 2] = ibapi::DATA_ADVISORY_CODES;

// 4.0 — let the type follow the const
let advisories = ibapi::DATA_ADVISORY_CODES;
```

## Behavioral changes

No code changes required, but observable at runtime:

- **Wrong wire format is an error, not a skip.** A text-framed message reaching a proto-only decoder fails the subscription with the new `Error::UnexpectedWireFormat` instead of being silently dropped — at server version 213+ that framing means the gateway broke protocol, and previously the subscription just yielded nothing. Wrong-*message-type* frames are still skipped, which is what shared channels need.
- **One-shot requests narrow to the message type they asked for.** A foreign frame surfaces as `Error::UnexpectedResponse` naming both the expected and received type, instead of being fed to the wrong payload decoder — where overlapping protobuf field numbers usually produced a plausible struct full of wrong values.
- **Retry-on-reset is uniform.** `market_rule`, `family_codes`, `calculate_option_price`, `calculate_implied_volatility`, and `next_valid_order_id` now retry a connection reset up to three times like every other one-shot; `head_timestamp`, `histogram_data`, `market_depth_exchanges`, `historical_data(..).fetch()`, and `historical_schedules(..).fetch()` now retry *at most* three times instead of unboundedly (or, on the async side, not at all), and sync/async agree on what a closed stream returns.
- **Frames are validated.** A length prefix that cannot describe a TWS message (shorter than the message id, or over the official 16 MiB cap) raises `Error::InvalidFrame` and drives a reconnect instead of a multi-gigabyte allocation and a permanently mis-framed stream; a body too short for the message id no longer panics the dispatcher. A frame whose message id maps to no known type raises an `UNKNOWN_MESSAGE_TYPE_CODE` (`-5`) notice on `Client::notice_stream` — the observable form of a framing desynchronization. Both new `Error` variants arrive via `#[non_exhaustive]`, so they are not compile-breaking.
- **Notices reclassified.** Codes 2188 and 10090 are data advisories (TWS keeps delivering data after sending them, but the subscription used to be torn down); code-399 order messages whose text carries a `Warning:` line classify as warnings instead of order rejections; a notice with no `error_code` field (code 0) classifies as a warning instead of failing every in-flight shared one-shot.
- **The order-update stream delivers order-bound errors as notices.** Order-bound error frames arrive as `SubscriptionItem::Notice` (with `request_id`, code, and message) instead of raw frames that failed to decode as `OrderUpdate`. Note `filter_data()` / `iter_data()` drop notices — match on `SubscriptionItem::Notice` to observe rejections of fire-and-forget orders. Request-less errors and errors owned by a data-request subscription no longer reach the stream at all.
- **Real errors instead of empty results.** `OrderBuilder::analyze()` returns the TWS rejection (e.g. code 201) instead of `Error::UnexpectedEndOfStream`; blocking `matching_symbols()` returns the TWS error instead of `Ok(vec![])`.
- **Malformed decimals fail instead of decoding as `0`.** Beyond the size fields whose types changed in [§1](#1-market-data-sizes-are-optionf64), every decimal-typed wire field — order quantities, execution shares, positions, bar volume/WAP, market-depth sizes — now surfaces a malformed value as `Error::Parse` instead of silently substituting `0`. TWS's "unset" sentinels are also recognized on all of these fields (previously only a few), decoding to `None` — or `0.0` where the field stays `f64` — instead of leaking as a literal 2.1-billion value.
- **`TickTypes::MarketDataType` actually arrives.** The variant existed but was never routed to `Client::market_data` subscriptions; TWS's market-data-type notifications (real-time / frozen / delayed / delayed-frozen, sent on subscribe and whenever the feed switches) now reach them, so a match that never saw this variant will start seeing it.
- **Reconnection is configurable and more resilient.** `ClientBuilder::max_reconnect_attempts` / `reconnect_forever` control the retry budget (default unchanged: 20 attempts, ~7.5 minutes). A session-establishment failure (handshake, `startAPI`, account info) consumes one attempt and backs off instead of aborting the loop — common during an automated TWS restart — and when every attempt fails, `reconnect` returns the last real error instead of a generic `Error::ConnectionFailed`.

## Quick migration checklist

1. Add a `Liquidity::Unknown(code)` arm to exhaustive matches on `Execution.last_liquidity` — the compiler finds them for you.
2. Unwrap market-data sizes: historical tick / histogram sizes and the `ContractDetails` size rules are `Option<f64>`. Watch for `max_by_key` on a size (`f64` isn't `Ord`), `+=` into an integer accumulator, and `{}` formatting — see [§1](#1-market-data-sizes-are-optionf64).
3. Rewrite `wsh_event_data_by_contract` / `wsh_event_data_by_filter` calls as builder chains ending in `.fetch()` / `.subscribe()`.
4. Replace `client.check_server_version(..)` with a comparison against `client.server_version()`.
5. Add `request_id: None` (or a real id) to any `Notice` struct literals.
6. If you consume the order-update stream through `filter_data()` / `iter_data()`, decide whether you need a `SubscriptionItem::Notice` arm to observe order rejections.
7. If you serialize market-data types to JSON, update downstream consumers: sizes are now `number | null` instead of `integer`, and notices may carry `request_id`.
8. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and your test suite for each feature flag you support.

## Need help?

- Examples: `examples/async` and `examples/sync`
- README: [Handling notifications](../README.md#handling-notifications)
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
