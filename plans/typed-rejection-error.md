# Typed server rejections — `Error::Message` → `Error::Notice(Notice)`

**Status:** planned · **Parent:** [`plans/v3-api-ergonomics.md`](v3-api-ergonomics.md) §5 item 2 ("Distinguish 'request rejected by server' from 'transport error' in return types").

## Context

The streaming side already separates server rejections from transport errors: per-subscription notices arrive as `SubscriptionItem::Notice(Notice)` (PR #517), and `Notice::category()` / `is_order_rejection()` / `is_warning()` (PR #551) typed the classification.

The **sync RPC-style** return paths (`fn foo(...) -> Result<T, Error>`) didn't get the same treatment. ~25 production call sites surface server-side TWS error frames as `Err(Error::Message(i32, String))`. Callers can't separate "TWS rejected my request" from "the socket died" without inspecting the variant *and* string-parsing the message:

```rust
match client.contract_details(&c) {
    Ok(details) => ...,
    Err(Error::Message(code, msg)) => /* TWS said no — but is it a rejection? warning? farm-status? */,
    Err(Error::Io(_)) => /* transport */,
    Err(_) => /* ??? */,
}
```

Goal: make `Result<_, Error>` returns symmetric with the streaming path. A typed `Error::Notice(Notice)` variant carries the full typed wire frame (code, message, error_time, advanced_order_reject_json) and gives callers the same `n.category()` / `n.is_order_rejection()` taxonomy they already get from `SubscriptionItem::Notice`.

## Decision summary

- **Shape:** `Error::Notice(Notice)` replaces `Error::Message(i32, String)`.
- **Why Notice, not (code, message):** reuses the typed struct + `NoticeCategory` partition already built for the streaming side. Symmetric with `SubscriptionItem::Notice`. Preserves `error_time` and `advanced_order_reject_json` (today's `Error::Message` drops both — `From<ResponseMessage> for Error` only pulls `error_code()` + `error_message()`).
- **Pacing:** remove `Error::Message` in the same PR. v3.0 is the breaking-release line; this matches the PR #584–#590 audit cadence.
- **Slicing:** single PR. Mechanical sweep once the variant flips; mirrors PR #590's scope.

## Why this isn't `Error::ConnectionRejected`

`Error::ConnectionRejected(String)` (PR #590) is a **handshake-time** refusal — gateway accepted TCP, closed before completing the IB handshake (typically allow-list mismatch). `Error::Notice` is an **in-protocol** TWS error frame routed back through a request. Both legitimately exist; the new variant doesn't subsume the old.

| Layer | Variant | Trigger |
|---|---|---|
| TCP | `Error::Io(_)`, `Error::ConnectionFailed`, `Error::ConnectionReset` | socket-level failure |
| Handshake | `Error::ConnectionRejected(String)` | gateway closed pre-handshake |
| In-protocol | `Error::Notice(Notice)` *(new)* | `IncomingMessages::Error` frame on a live request |

## What changes

### `Error` enum

```rust
// before
#[error("[{0}] {1}")]
Message(i32, String),

// after
#[error("{0}")]
Notice(Notice),  // Notice's Display impl is already `[{code}] {message}`
```

The variant tuple drops; payload is the full typed `Notice` struct. `Display` output is unchanged (Notice's Display impl already prints `[code] message`).

### `From` impls in `errors.rs`

```rust
impl From<ResponseMessage> for Error {
    fn from(err: ResponseMessage) -> Error {
        Error::Notice(Notice::from(&err))   // was: Error::Message(err.error_code(), err.error_message())
    }
}

impl From<crate::transport::routing::DecodedError> for Error {
    fn from(payload: crate::transport::routing::DecodedError) -> Error {
        Error::Notice(Notice::from(payload))  // was: Error::Message(payload.error_code, payload.error_message)
    }
}
```

`Notice` already implements `From<&ResponseMessage>` (`messages.rs:1480`) and `From<DecodedError>` (`messages.rs:1647`), with the protobuf branch preserving `error_time` and `advanced_order_reject_json` — switching the projection picks both up for free.

`Clone` arm flips `Error::Message(c, m) => Error::Message(*c, m.clone())` → `Error::Notice(n) => Error::Notice(n.clone())` (`Notice` already derives `Clone`).

### Constructor sites — no production change needed

Every production site that produces a TWS-frame error currently writes `Err(Error::from(message))` or `RoutedItem::Error(Error::from(payload))`. The new `From` impls absorb the variant change; **the 25 call sites enumerated below don't need editing**.

Inventory (all currently `Err(Error::from(message))` or `Err(Error::from(message.clone()))`):

| File | Sites | Notes |
|---|---:|---|
| `contracts/sync/mod.rs` | 1 | `contract_details` |
| `contracts/async/mod.rs` | 1 | `contract_details` (line 60) |
| `contracts/common/stream_decoders.rs` | 2 | per-decoder error arms |
| `market_data/historical/{sync,async}.rs` | 2 | `historical_schedule` style return |
| `market_data/historical/mod.rs` | 1 | shared decoder error arm |
| `news/common/stream_decoders.rs` | 2 | per-decoder error arms |
| `accounts/common/stream_decoders/mod.rs` | 7 | per-decoder error arms |
| `wsh/common/stream_decoders.rs` | 2 | per-decoder error arms |
| `wsh/common/decoders.rs` | 1 | shared decoder error arm |
| `scanner/common/decoders.rs` | 1 | shared decoder error arm |
| `scanner/common/stream_decoders.rs` | 1 | per-decoder error arm |
| `display_groups/common/stream_decoders.rs` | 1 | per-decoder error arm |
| `transport/{async.rs,sync/mod.rs}` | 2 | dispatcher `RoutedItem::Error(Error::from(payload))` |

(Plus the two inline `contracts/{sync,async}` paths at `contracts/sync/mod.rs:52` / `contracts/async/mod.rs:60` that match the `IncomingMessages::Error` arm inline.)

### Pattern-match sites — flip required

These read or build `Error::Message(...)` directly and must move to `Error::Notice(_)`:

**Production (3 sites):**

- `subscriptions/sync.rs:176` — terminal-error `warn!`. Flip to `Error::Notice(n) => warn!("subscription terminated by TWS error {n}")` (Notice's Display already has `[code] message`).
- `client/error_handler/mod.rs:70` — `error_message()` helper. **Preserve exact format**: old `format!("TWS Error [{code}]: {msg}")` → new `Error::Notice(n) => format!("TWS Error [{}]: {}", n.code, n.message)`. The naive `format!("TWS Error {n}")` would drop the colon (Notice's Display is `[code] message`, not `[code]: message`) and break `client/error_handler/tests.rs:315` which asserts the exact string `"TWS Error [200]: test error"`.
- `client/error_handler/mod.rs:96` — `categorize_error()` `ServerError` arm. Flip to `Error::Notice(_) => ErrorCategory::ServerError`.

**Test infra (1 site, public-ish helper):**

- `common/test_utils.rs::assert_tws_error_message` — `Error::Message(code, msg)` match. Flip body to destructure `Error::Notice(Notice { code, message, .. })`. Doc comment updated. Helper name keeps `_tws_error_message` (matches the wire concept). **The panic-message literal must update in lockstep** — `panic!("expected Error::Message({expected_code}, _), got {other:?}")` → `panic!("expected Error::Notice(code={expected_code}), got {other:?}")` — and the corresponding `#[should_panic(expected = "expected Error::Message(10089, _)")]` attribute at `common/test_utils_tests.rs:140` becomes `#[should_panic(expected = "expected Error::Notice(code=10089)")]` (or whatever exact string the new panic format produces). Three-way coupling: helper body string, attribute literal, and `should_panic` substring match must all agree.

**Test files (~10 sites):**

- `errors_tests.rs` × 5 — variant-construction + display-assertion. Use `crate::messages::Notice::synthesized(code, msg.into())` (it's `pub(crate)`, accessible from sibling test file) instead of struct-literals for brevity. Two tests get **renamed** to track the variant: `from_decoded_error_moves_into_message_variant` → `..._into_notice_variant`; the `error_display` table at line 57 keeps the same assertion string (`"[200] No security found"`) since `Error::Notice` Display delegates to `Notice` Display which is already `[code] message`.
- One subtle case: `from_protobuf_response_message_falls_back_when_decode_fails` (errors_tests.rs:152) asserts `matches!(error, Error::Message(0, _))` when bad bytes can't decode and the path falls back to text accessors. The new variant: `matches!(error, Error::Notice(n) if n.code == 0)`. **Semantics are identical** because `Notice::from(&ResponseMessage)` on the protobuf branch falls back to the same `error_code()` / `error_message()` text accessors that the old `Error::from` chain used; this is mechanical, not behavioral.
- `transport/routing_tests.rs:58-78` — `test_error_from_decoded_projects_to_message` test + its match arm. **Rename** to `..._projects_to_notice`; flip the match.
- `transport/async_tests.rs:262,384,388` — dispatcher tests + post-routing assertion.
- `transport/sync/tests.rs:937,1066,1070` — sync mirror of the above.
- `wsh/{sync,async}_tests.rs:331,187` — `matches!(err, Error::Message(_, _))`.
- `client/error_handler/tests.rs:314,415` — error_handler test fixtures. The `error_message` table at 314 also asserts the exact string `"TWS Error [200]: test error"` — preserved verbatim by the format-spelling-out in `mod.rs:70` above.
- `common/test_utils_tests.rs:135,140,149,156` — `assert_tws_error_message` test cases + the `#[should_panic(expected = ...)]` literal noted above.

**Integration crates (4 sites):**

- `integration/async/tests/algo_orders.rs:56-57`
- `integration/async/tests/conditional_orders.rs:40-41`
- `integration/sync/tests/algo_orders.rs:57-58`
- `integration/sync/tests/conditional_orders.rs:44-48`

Pattern in each: `Err(Error::Message(201, msg)) => panic!(...)` + `Err(Error::Message(_, _)) => /* warn */`. Flip to `Err(Error::Notice(n)) if n.code == 201 => panic!(...)` + `Err(Error::Notice(_)) => /* warn */`. Comments referencing "Error::Message" updated too.

### Doc-comment references (~6 sites)

Search-and-replace `Error::Message` → `Error::Notice` in:

- `display_groups/common/stream_decoders.rs:87`
- `contracts/common/stream_decoders.rs:72`
- `wsh/common/stream_decoders.rs:63`
- `news/common/stream_decoders_tests.rs:14`
- `accounts/common/stream_decoders/tests.rs:45`
- `scanner/common/stream_decoders.rs:37`
- `errors.rs:135` (doc on `From<DecodedError>` impl) — also update the doc-comment body since it currently says "to `Error::Message`".

Grep `Error::Message` after the sweep — zero hits is the gate (caveat: this plan file itself contains `Error::Message` strings for explanatory purposes; exclude `plans/**` from the gate grep).

### `docs/migration-3.0.md`

Add an §N entry under the existing Errors discussion:

```md
### Error::Message → Error::Notice(Notice)

TWS-emitted error frames now arrive as `Error::Notice(Notice)` instead of
`Error::Message(i32, String)`. The new variant carries the full typed
notice (code, message, error_time, advanced_order_reject_json) and
exposes the same classification API as the streaming side:

```rust
// before
match err {
    Error::Message(200..=399, msg) => log::warn!("rejection: {msg}"),
    ...
}

// after
match err {
    Error::Notice(n) if n.is_order_rejection() => log::warn!("rejection: {n}"),
    Error::Notice(n) => match n.category() { ... },
    ...
}
```

This makes `Result<_, Error>` returns symmetric with `Subscription<T>`
items (which already yield `SubscriptionItem::Notice(Notice)` for the
same wire frame). It is distinct from `Error::ConnectionRejected`
(handshake-time refusal) and the transport variants (`Error::Io`,
`Error::ConnectionReset`).
```

### `README.md`

Grep for `Error::Message` — should be empty (verified above). If a match shows up, flip.

## Test additions

Beyond flipping the existing assertions:

- `errors_tests.rs` — round-trip test for `Error::Notice` Display matches `Notice` Display (no `[code]` doubling). Test that `From<ResponseMessage>` preserves `error_time` and `advanced_order_reject_json` on the protobuf path. Test that `Clone` of `Error::Notice` round-trips by value.
- `errors_tests.rs` — taxonomy test: build `Error::Notice` from synthesized notices at category boundaries (200, 202, 399, 1100, 2100, 2169, 10000) and assert `matches!(err, Error::Notice(n) if n.category() == NoticeCategory::X)` for each. Anchors the documented partition at the `Error` layer.
- `common/test_utils_tests.rs` — add a third case: `assert_tws_error_message` on a notice with non-empty `advanced_order_reject_json` succeeds (verifies the helper still works when the notice carries the new fields).

## Verification

Per the standard sweep (CLAUDE.md "Quick Commands"):

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
just test
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
```

Last grep gate before opening the PR:

```bash
git grep -n 'Error::Message' -- ':!plans/**'   # must be empty
git grep -n 'error::Message'  -- ':!plans/**'  # must be empty
```

No live-gateway test required — every site is constructor-side or pattern-match-side; routing/decode paths are exercised by existing unit tests.

## Out of scope

- Promoting any *other* `Error::Simple` site — the audit ([`plans/error-variants-audit.md`](error-variants-audit.md)) is complete.
- Reshaping `Notice` itself (e.g. typing `code` as an enum) — see `plans/typed-status-sweep.md` for that workstream; the new `Error::Notice(Notice)` variant inherits whatever shape `Notice` carries.
- Removing `Error::ConnectionRejected(String)` — it remains the canonical handshake-refusal signal; this plan is strictly about in-protocol TWS error frames.

## Risk / rollback

- **Risk:** downstream code outside this repo that pattern-matches `Error::Message(_, _)` breaks. Mitigated by: (a) the variant rename is the v3.0 breaking change, called out in `docs/migration-3.0.md`; (b) `cargo check` on downstream crates surfaces the missing variant arm at the call site (named pattern, not exhaustiveness — `Error` is `#[non_exhaustive]`, so `_` arms keep compiling; named arms `Error::Message(_, _)` fail with a clear "no variant named `Message`" error).
- **Rollback:** revert the single PR. The constructor sites are all on the `From` impls, so re-introducing `Error::Message(code, msg)` is one file's change.
