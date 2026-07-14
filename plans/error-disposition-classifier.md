# Plan: Centralize request-less error policy in a shared `ErrorDisposition` classifier (issue #700)

## Problem

The *policy* for classifying an inbound error frame is duplicated verbatim across both transports:

- `src/transport/sync.rs` — `fn route_error_message(&self, …, payload: DecodedError)`
- `src/transport/async.rs` — `async fn route_error_message(&self, …, payload: DecodedError)`

Both encode the same four-arm decision:

| `request_id` | `is_warning` | action |
|---|---|---|
| `-1` | yes | log + broadcast to `NoticeStream` |
| `-1` | no  | log + broadcast to `NoticeStream` + fail-fast one-shot shared channels |
| `!= -1` | yes | deliver `RoutedItem::Notice` to owning subscription |
| `!= -1` | no  | deliver `RoutedItem::Error` to owning subscription |

The delivery mechanics (crossbeam vs tokio broadcast, `RwLock.read().await` vs direct map access) are legitimately per-runtime (CLAUDE.md rule 13). The **policy** — deciding which of the four arms applies — is not. Any future tweak (e.g. a code-specific exemption like the 10089/10167 informational case, or expanding what "warning" means for unrouted errors) must currently be applied twice or the transports silently diverge.

## Scope

Pure refactor — no public API change, no behavior change. No `CHANGELOG.md` entry required.

## Design

Add `ErrorDisposition` and `classify_error` to `src/transport/routing.rs`, which already owns `determine_routing` and `is_warning_error`. Each transport's `route_error_message` becomes a thin `match` that only performs runtime-specific delivery.

```rust
// src/transport/routing.rs

pub(crate) enum ErrorDisposition {
    /// Log + NoticeStream only (request-less warning).
    NoticeOnly(Notice),
    /// NoticeStream + fail-fast fan-out to one-shot shared channels
    /// (request-less hard error).
    NoticeAndFailOneShots(Notice, Error),
    /// Deliver to the owning subscription by request/order id.
    Route(i32, RoutedItem),
}

pub(crate) fn classify_error(payload: DecodedError) -> ErrorDisposition {
    let request_id = payload.request_id;
    let is_warning  = is_warning_error(payload.error_code);

    if request_id == UNSPECIFIED_REQUEST_ID {
        let notice = Notice::from(payload.clone());
        if is_warning {
            ErrorDisposition::NoticeOnly(notice)
        } else {
            ErrorDisposition::NoticeAndFailOneShots(notice, Error::from(payload))
        }
    } else {
        let item = if is_warning {
            RoutedItem::Notice(Notice::from(payload))
        } else {
            RoutedItem::Error(Error::from(payload))
        };
        ErrorDisposition::Route(request_id, item)
    }
}
```

### Why `Error` in `NoticeAndFailOneShots` works

`Error` has a manual `Clone` implementation (verified in `src/errors.rs:239`). The sync `fail_one_shot_channels` helper takes `Fn() -> RoutedItem` to support fan-out to multiple channels; a `|| RoutedItem::Error(error.clone())` closure handles this cleanly without storing `DecodedError`.

## Files to change

| File | Change |
|---|---|
| `src/transport/routing.rs` | Add imports; add `ErrorDisposition` enum; add `classify_error` function |
| `src/transport/sync.rs` | Import `classify_error`, `ErrorDisposition`; rewrite `route_error_message` body |
| `src/transport/async.rs` | Import `classify_error`, `ErrorDisposition`; rewrite `route_error_message` body; update `fail_one_shot_channels` signature |
| `src/transport/routing_tests.rs` | Add four unit tests for `classify_error` |

No other files need changing. The existing transport tests (`test_request_less_warning_does_not_fail_one_shot`, `test_unrouted_hard_error_*`, `test_warning_with_order_id_falls_back_to_order_channel`, etc.) assert end-to-end routing behaviour and must pass unchanged.

---

## Step-by-step implementation

### Step 1 — `src/transport/routing.rs`: add imports + new items

**Imports to add** at the top of the file (alongside the existing `messages` import):

```rust
use crate::errors::Error;
use crate::messages::{IncomingMessages, Notice, ResponseMessage, DATA_ADVISORY_CODES, WARNING_CODE_RANGE};
use crate::subscriptions::common::RoutedItem;
```

*(The existing import already has `IncomingMessages` and `ResponseMessage`; replace it with the expanded form.)*

**Add after `is_warning_error`:**

```rust
/// The outcome of classifying an inbound error frame.
///
/// The *policy* (which arm applies) is centralised here; each transport
/// provides only the runtime-specific delivery in its `route_error_message`.
#[derive(Debug)]
pub(crate) enum ErrorDisposition {
    /// Log + `NoticeStream` only (request-less warning).
    NoticeOnly(Notice),
    /// `NoticeStream` + fail-fast fan-out to in-flight one-shot shared
    /// requests (request-less hard error).
    NoticeAndFailOneShots(Notice, Error),
    /// Deliver `RoutedItem` to the subscription that owns `request_id`.
    Route(i32, RoutedItem),
}

/// Classify an inbound error frame into the action each transport must take.
///
/// Extracts the common four-arm policy so that `sync::route_error_message` and
/// `async::route_error_message` are thin runtime-specific delivery shells.
pub(crate) fn classify_error(payload: DecodedError) -> ErrorDisposition {
    let request_id = payload.request_id;
    let is_warning = is_warning_error(payload.error_code);

    if request_id == UNSPECIFIED_REQUEST_ID {
        let notice = Notice::from(payload.clone());
        if is_warning {
            ErrorDisposition::NoticeOnly(notice)
        } else {
            ErrorDisposition::NoticeAndFailOneShots(notice, Error::from(payload))
        }
    } else {
        let item = if is_warning {
            RoutedItem::Notice(Notice::from(payload))
        } else {
            RoutedItem::Error(Error::from(payload))
        };
        ErrorDisposition::Route(request_id, item)
    }
}
```

---

### Step 2 — `src/transport/sync.rs`: thin `route_error_message`

**In the `use super::routing::{...}` import block**, add `classify_error` and `ErrorDisposition`:

```rust
use super::routing::{
    classify_error, determine_routing, is_warning_error, order_routing_strategy,
    DecodedError, ErrorDisposition, OrderRoutingStrategy, RoutingDecision, UNSPECIFIED_REQUEST_ID,
};
```

*(Remove `is_warning_error` and `UNSPECIFIED_REQUEST_ID` from the import if they are no longer referenced directly in `route_error_message` — check for other call sites first.)*

**Replace the body of `route_error_message`:**

```rust
fn route_error_message(&self, message: &ResponseMessage, payload: DecodedError) {
    let sent_to_update_stream = self.send_order_update(message);
    match classify_error(payload) {
        ErrorDisposition::NoticeOnly(notice) => {
            super::common::log_unrouted_notice(&notice);
            self.connection.notice_broadcaster.broadcast(notice);
        }
        ErrorDisposition::NoticeAndFailOneShots(notice, error) => {
            super::common::log_unrouted_notice(&notice);
            self.connection.notice_broadcaster.broadcast(notice);
            self.shared_channels
                .fail_one_shot_channels(|| RoutedItem::Error(error.clone()));
        }
        ErrorDisposition::Route(request_id, item) => {
            self.deliver_to_request_id(request_id, item, sent_to_update_stream);
        }
    }
}
```

**Verify** that `is_warning_error` and `UNSPECIFIED_REQUEST_ID` still appear elsewhere in `sync.rs`; if not, remove them from the import to keep it lean.

---

### Step 3 — `src/transport/async.rs`: thin `route_error_message` + updated `fail_one_shot_channels`

**In the `use super::routing::{...}` import block**, add `classify_error` and `ErrorDisposition`:

```rust
use super::routing::{
    classify_error, determine_routing, is_warning_error, order_routing_strategy,
    DecodedError, ErrorDisposition, OrderRoutingStrategy, RoutingDecision, UNSPECIFIED_REQUEST_ID,
};
```

**Replace `route_error_message`:**

```rust
async fn route_error_message(&self, message: ResponseMessage, payload: DecodedError) -> Result<(), Error> {
    let sent_to_update_stream = self.send_order_update(&message).await;
    match classify_error(payload) {
        ErrorDisposition::NoticeOnly(notice) => {
            super::common::log_unrouted_notice(&notice);
            let _ = self.connection.notice_sender.send(notice);
        }
        ErrorDisposition::NoticeAndFailOneShots(notice, error) => {
            super::common::log_unrouted_notice(&notice);
            let _ = self.connection.notice_sender.send(notice);
            self.fail_one_shot_channels(error).await;
        }
        ErrorDisposition::Route(request_id, item) => {
            self.deliver_to_request_id(request_id, item, sent_to_update_stream).await;
        }
    }
    Ok(())
}
```

**Update `fail_one_shot_channels`** to take a pre-built `Error` instead of `&DecodedError` (the caller already has the `Error` from `classify_error`):

```rust
/// Deliver a request-less hard error to every in-flight one-shot shared
/// request so it fails fast rather than hanging. Streaming shared channels
/// are excluded (see [`shared_channel_configuration::exclusive_one_shot_response_types`]).
async fn fail_one_shot_channels(&self, error: Error) {
    let channels = self.shared_channel_senders.read().await;
    for message_type in shared_channel_configuration::exclusive_one_shot_response_types() {
        if let Some(senders) = channels.get(message_type) {
            for sender in senders {
                let _ = sender.send(RoutedItem::Error(error.clone()));
            }
        }
    }
}
```

**Verify** that `is_warning_error` and `UNSPECIFIED_REQUEST_ID` still appear elsewhere in `async.rs`; if not, remove them from the import.

---

### Step 4 — `src/transport/routing_tests.rs`: add `classify_error` tests

Add after the existing `is_warning_error`-related tests (or at the end of the file):

```rust
// ---- classify_error tests ------------------------------------------------

#[test]
fn test_classify_error_unrouted_warning_is_notice_only() {
    // request_id == -1, is_warning == true  →  NoticeOnly
    let payload = DecodedError {
        request_id: UNSPECIFIED_REQUEST_ID,
        error_code: 2104,  // within WARNING_CODE_RANGE
        error_message: "Market data farm OK".into(),
        ..DecodedError::default()
    };
    match classify_error(payload) {
        ErrorDisposition::NoticeOnly(notice) => {
            assert_eq!(notice.code, 2104);
            assert_eq!(notice.message, "Market data farm OK");
        }
        other => panic!("expected NoticeOnly, got {other:?}"),
    }
}

#[test]
fn test_classify_error_unrouted_hard_is_notice_and_fail_one_shots() {
    // request_id == -1, is_warning == false  →  NoticeAndFailOneShots
    let payload = DecodedError {
        request_id: UNSPECIFIED_REQUEST_ID,
        error_code: 321,  // outside WARNING_CODE_RANGE — hard error
        error_message: "Server error".into(),
        ..DecodedError::default()
    };
    match classify_error(payload) {
        ErrorDisposition::NoticeAndFailOneShots(notice, error) => {
            assert_eq!(notice.code, 321);
            assert_eq!(notice.message, "Server error");
            // Error must project the same notice payload
            match error {
                crate::Error::Notice(n) => assert_eq!(n.code, 321),
                other => panic!("expected Error::Notice, got {other:?}"),
            }
        }
        other => panic!("expected NoticeAndFailOneShots, got {other:?}"),
    }
}

#[test]
fn test_classify_error_routed_warning_is_route_notice() {
    // request_id != -1, is_warning == true  →  Route(id, RoutedItem::Notice)
    let payload = DecodedError {
        request_id: 42,
        error_code: 2104,
        error_message: "Farm OK".into(),
        ..DecodedError::default()
    };
    match classify_error(payload) {
        ErrorDisposition::Route(id, RoutedItem::Notice(notice)) => {
            assert_eq!(id, 42);
            assert_eq!(notice.code, 2104);
        }
        other => panic!("expected Route(42, Notice), got {other:?}"),
    }
}

#[test]
fn test_classify_error_routed_hard_is_route_error() {
    // request_id != -1, is_warning == false  →  Route(id, RoutedItem::Error)
    let payload = DecodedError {
        request_id: 7,
        error_code: 200,
        error_message: "No security".into(),
        ..DecodedError::default()
    };
    match classify_error(payload) {
        ErrorDisposition::Route(id, RoutedItem::Error(error)) => {
            assert_eq!(id, 7);
            match error {
                crate::Error::Notice(n) => assert_eq!(n.code, 200),
                other => panic!("expected Error::Notice payload, got {other:?}"),
            }
        }
        other => panic!("expected Route(7, Error), got {other:?}"),
    }
}
```

Note: `classify_error` and `ErrorDisposition` must be re-exported from `routing.rs` into `routing_tests.rs` via the existing `use super::*;` wildcard already present in the test file.

---

## Verification checklist

Run these before opening the PR:

```bash
cargo fmt

cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features

RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

just test
```

All existing transport tests (sync + async) must pass without modification. The four new `classify_error` unit tests in `routing_tests.rs` are the only new tests required.

## What does not change

- `RoutingDecision` and `determine_routing` — untouched
- `DecodedError`, `decode_error_envelope` — untouched  
- `deliver_to_request_id` (both transports) — untouched
- `send_order_update` (both transports) — untouched; called before `classify_error` in each transport, result passed to `Route` arm
- All public API types and trait impls — no user-visible change
- `CHANGELOG.md` — no entry needed (internal refactor per issue #700)
