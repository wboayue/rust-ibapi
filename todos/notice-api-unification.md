# Notice API Unification — v3.0 Follow-Up

**Status:** SHIPPED 2026-05-08 (option 3, folded with `Client::builder()` per
v3-api-ergonomics §4.1). The original recommendation was option A (smallest
shippable, `ConnectionOptions::with_notice_stream`); the implementation took
option 3 directly because it costs a bigger diff but fixes the lifecycle gap,
the `--all-features` naming gymnastics, AND the three-entry-point sprawl in
one move. Hard-removed: `ConnectionOptions`, `StartupMessageCallback`,
`StartupNoticeCallback`, `Client::connect_with_options`, and
`Client::connect_with_callback`. Kept: `Client::connect(addr, id)` one-liner.

Spawned out of the 2026-05-06 integration-health pass while investigating the
`startup_notice_callback_receives_handshake_notices` flake (root cause: the
trailing-handshake-notice race after `(NextValidId && ManagedAccounts)` exits
the loop). The race is fixed by routing handshake notices through the
broadcaster, which now lives on `Connection` and is reused across reconnects.

## Problem

Two parallel public APIs deliver the same thing — unrouted notices broadcast
by the gateway (farm-status 2104/2106/2158, connectivity 1100/1101/1102, etc.):

1. **`ConnectionOptions::startup_notice_callback(...)`** — set pre-connect.
   Fires only for notices read inside the handshake loop in
   `Connection::receive_account_info`.
2. **`Client::notice_stream() -> NoticeStream`** — post-connect. Subscribes to
   the bus's `NoticeBroadcaster`, which receives every unrouted notice for the
   lifetime of the connection.

Lifecycle gap: callback covers the handshake window, stream covers
post-connect. A user who wants both has to wire both. Worse, the callback's
window is racy — if the gateway emits `ManagedAccounts` before all handshake
notices, the loop exits and the trailing notices land on the bus where the
callback isn't wired.

## Today's surfaces

- `src/connection/common.rs:103` — `ConnectionOptions.startup_notice_callback`
  field
- `src/connection/common.rs:134` — `ConnectionOptions::startup_notice_callback`
  builder
- `src/connection/sync.rs:35` / `src/connection/async.rs:36` — stored on
  `Connection` / `AsyncConnection`
- `src/connection/common.rs:306` (`dispatch_unsolicited_message`) — fires the
  callback for `IncomingMessages::Error`
- `src/transport/sync/mod.rs:309` — bus dispatch path that broadcasts to
  `NoticeBroadcaster` (`Client::notice_stream()` consumes from here)
- `src/transport/sync/mod.rs:641` — `notice_subscribe()` returns the
  `NoticeStream`

## Design question

Pick one canonical API. Either drop the other or make it a thin convenience
over the survivor.

## Options

- **(A) Keep stream, drop callback.** Plumb a pre-connect `NoticeStream` handle
  through `ConnectionOptions` (e.g. `options.notice_subscription()` returns the
  stream and stashes the receiver, the bus picks it up at construction).
  - Pro: streams already match the codebase idiom (`Subscription`,
    `NoticeStream`, `OrderUpdateStream` are all post-connect, all
    Subscription-shape).
  - Con: extra plumbing through `ConnectionOptions` to share the broadcaster
    instance between caller and the eventually-constructed bus.

- **(B) Keep callback, drop stream.** Forward all unrouted notices through the
  same callback for the lifetime of the connection.
  - Pro: trivial to implement (callback already exists).
  - Con: side-effects-only; async users will reinvent a channel inside the
    callback to get stream ergonomics. No backpressure or cancellation.

- **(C) Status quo + bridge.** Make the callback a thin sugar over the
  broadcaster; both APIs remain but route through one source of truth.
  - Pro: no external behavior break, fixes the handshake race for free.
  - Con: doesn't simplify the public surface — the redundancy stays.

## Recommendation

Option **(A)**: keep the stream, drop the callback. Streams compose with the
rest of the project's Subscription pattern, and the chicken-and-egg of "can't
subscribe until after `connect()`" is solvable by handing the caller a
pre-bound `NoticeStream` from `ConnectionOptions`.

## Design sketch (option A)

### User-facing API

```rust
// async (default feature)
let (options, mut notices) = ConnectionOptions::default()
    .tcp_no_delay(true)
    .with_notice_stream();

let client = Client::connect_with_options("127.0.0.1:4002", 100, options).await?;

while let Some(n) = notices.next().await {
    println!("{n}");
}
```

`with_notice_stream(self) -> (Self, NoticeStream)` is a consuming method on
`ConnectionOptions` — it allocates a broadcaster, subscribes once, returns the
options (with the broadcaster stashed inside) and the stream. Same shape on
sync, just no `.await` on the consumer side.

### Internal plumbing

The broadcaster moves up the stack so handshake and bus share it.

- `ConnectionOptions` gains an `Option<NoticeBroadcaster>` field (replacing
  `startup_notice_callback`).
- `Connection::connect_with_options` takes the broadcaster from `options`. If
  `None`, it creates one — so plain `connect()` users still get a working
  `client.notice_stream()` post-connect.
- `Connection` stores the broadcaster instead of `notice_callback`.
  `dispatch_unsolicited_message` (`src/connection/common.rs:306`) pushes onto
  the broadcaster instead of calling a callback. `StartupCallbacks` shrinks:
  drop the `notice: Option<&dyn Fn>` field; the broadcaster is reachable from
  `Connection` directly.
- `TcpMessageBus::new(connection)` (sync `src/transport/sync/mod.rs:159`,
  async equivalent in `src/transport/async.rs`) extracts the broadcaster from
  `connection` instead of constructing one. Existing `client.notice_stream()`
  ↔ `bus.notice_subscribe()` paths don't change — same broadcaster, more
  subscribers.

The handshake-race fix happens for free: notices that arrive after the loop's
`(NextValidId && ManagedAccounts)` break still flow into the broadcaster via
the bus dispatcher path at `src/transport/sync/mod.rs:309`, so the user's
pre-bound stream sees them.

### The dual-feature wrinkle

`NoticeBroadcaster` differs across features (sync = `Mutex<Vec<crossbeam
Sender>>` with manual prune; async = `tokio::sync::broadcast::Sender<Notice>`
direct, per the no-wrapper rule for tokio primitives). `ConnectionOptions`
lives in shared `connection/common.rs`, so the field can't reference a
per-feature type cleanly. Three ways out:

1. **Two cfg-gated fields** on `ConnectionOptions`
   (`sync_notice_broadcaster` + `async_notice_broadcaster`). Smallest change;
   ugly but localized — only `with_notice_stream` and the Connection
   extractors need cfg arms.
2. **Trait-object sink** (`Option<Arc<dyn NoticeSink>>` in options). Clean
   interior; trivial dyn cost; trait gets impls per feature.
3. **Defer to the `Client::builder()` work in v3-api-ergonomics §4.1.**
   Per-feature builders use native broadcaster types, no shared options
   struct to compromise. Cleanest but blocks on §4.1.

Recommend **(1)** as the smallest independently-shippable PR — matches the
existing dual-feature precedent for `Subscription`/`NoticeStream` and doesn't
depend on the bigger builder refactor. **(3)** is the right end state.

### Breaking-change surface

Removed public items:

- `ConnectionOptions::startup_notice_callback` field
- `ConnectionOptions::startup_notice_callback(...)` builder method
- `StartupNoticeCallback` type alias
- The `notice_callback` storage on `Connection` / `AsyncConnection` (private,
  but visible in test fixtures — `src/connection/sync_tests.rs:133`,
  `src/connection/async_tests.rs:126`)
- The `notice` field on `StartupCallbacks` (private)

Migration: replace

```rust
let options = ConnectionOptions::default()
    .startup_notice_callback(|n| println!("{n}"));
```

with

```rust
let (options, mut stream) = ConnectionOptions::default().with_notice_stream();
tokio::spawn(async move {
    while let Some(n) = stream.next().await { println!("{n}"); }
});
```

Kept: `Client::notice_stream()` works as today (subscribes to the same
broadcaster post-connect).

Files needing updates: `README.md` (any `startup_notice_callback` example),
`docs/migration-3.0.md` (new entry), the connection integration tests
(`integration/{sync,async}/tests/connection.rs:75`+), the synthetic-fixture
tests in `src/connection/{sync,async}_tests.rs`, `src/lib.rs:73` if it
mentions the callback type alias.

## Breaking?

Yes — removing one of the public surfaces is breaking. Acceptable per
[`CLAUDE.md`](../CLAUDE.md) §"Version 3.0 Philosophy" ("Fix API
inconsistencies even when it means breaking changes").

## Related

- The handshake-race fix (drain trailing handshake notices, or forward unrouted
  notices through the bus to the existing callback) is independent. Option
  (C) would resolve it as a side effect; (A) and (B) need it addressed
  separately.
- See [`v3-api-ergonomics.md`](v3-api-ergonomics.md) §2 "Streaming surface" for
  the broader subscription-shape decisions this folds into.
