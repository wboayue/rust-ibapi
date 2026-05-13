# Hide Internal Types From the Public Surface

**End goal:** the `ibapi` 3.0 public surface only exposes types and modules
users need; wire-plumbing (raw proto bindings, response framing, parser
registry) is `pub(crate)` or `#[doc(hidden)]`. After this sweep,
`cargo doc --no-deps` for an external consumer shows the user-facing API,
not the dispatcher.

**Parent:** [v3-api-ergonomics.md §3 "Hide internal types from the public surface"](v3-api-ergonomics.md).

## Audit (2026-05-12, against `main`)

| # | Item | Today | External consumers | Decision |
|---|---|---|---|---|
| 1 | `Client::stubbed()` async | `pub` + `#[cfg(test)]` (`src/client/async.rs:342`) | none (gated out of downstream builds) | → `pub(crate)`; match sync (`src/client/sync.rs:357`) |
| 2 | `Client::message_bus()` async | `pub` + `#[cfg(test)]` (`src/client/async.rs:359`) | none (same gating) | → `pub(crate)`; sync has no equivalent accessor |
| 3 | `pub mod proto` | `src/lib.rs:130` — generated prost bindings + `proto::{encoders,decoders}` helpers | zero hits in `examples/`, `integration/`, `docs/`, `README.md` | → `pub(crate) mod proto;` |
| 4 | `pub mod messages` | `src/lib.rs:110` — mixed: a few legitimately-public types, many wire internals | `examples/record_interactions.rs` reaches `parser_registry::*` + the message-id enums | split: re-export user-facing types from crate root + prelude; demote `messages` to `#[doc(hidden)] pub` (escape hatch for the recording example) |
| 5 | `subscriptions::common::SubscriptionItem` | `pub` inside `pub(crate) mod common` (`src/subscriptions/common.rs:18`); re-exported (`src/subscriptions/mod.rs:4`) | legitimate public API | **no change** |
| 6 | `subscriptions::common::DecoderContext` / `StreamDecoder` | `pub`/`pub(crate)` inside `pub(crate) mod common`; re-exported `pub(crate)` (`src/subscriptions/mod.rs:5`) | none (already crate-private from downstream's view) | **no change** |

`Notice` is not currently in the prelude (`src/prelude.rs:21-52`) and is
reachable only as `ibapi::messages::Notice`. PR 3 fixes this in the same
sweep — without a replacement path, demoting `messages` would be a breaking
removal with no migration target.

## Why this matters

The raw protobuf types are the wire format; any upstream `.proto` field
rename becomes a breaking change for any downstream consumer who imported
them. The `messages` module mixes that wire concern with legitimately-public
types (`Notice`, message-id enums, code-range constants), so today's
`cargo doc --no-deps` external view surfaces dispatcher internals next to
`Client`/`Contract`/`Order`. The async/sync `stubbed` asymmetry (#1, #2) is
unintentional drift — sync established the precedent in `pub(crate)`,
async didn't follow.

## PR breakdown

PRs ship independently. PR 1 and PR 2 are commutable; PR 3 lands last (largest
diff, most migration-guide surface). PR 4 is a no-code verification checkpoint
that may not need to exist if 1–3 leave nothing to clean up.

### PR 1 — async `stubbed` / `message_bus` → `pub(crate)`

Smallest, mechanical, restores sync/async parity. Items #1 and #2 in the
audit. No migration guide entry (gated `#[cfg(test)]`, never on the external
surface). Two attribute edits in `src/client/async.rs`.

### PR 2 — `pub mod proto` → `pub(crate) mod proto`

Item #3 in the audit. Zero external consumers, so the diff is a single line
in `src/lib.rs:130` plus narrowing the two child `pub mod` declarations in
`src/proto/mod.rs:5,7` to `pub(crate) mod`. All ~200 internal callsites use
`crate::proto::*` and are unaffected.

**Migration guide entry** (`docs/migration-3.0.md`):

> #### `ibapi::proto` is no longer public
>
> The raw protobuf wire types and their encoders/decoders were never
> intended as a stable surface. Consume the domain types (`Contract`,
> `Order`, `Execution`, …) directly. If you need a conversion path,
> file an issue.

### PR 3 — sort and trim `pub mod messages`

Item #4 in the audit. Three coupled outcomes, one PR:

**(a) Re-export user-facing types from crate root + prelude.**
`Notice`, `NoticeCategory`, `IncomingMessages`, `OutgoingMessages`,
`WARNING_CODE_RANGE`, `SYSTEM_MESSAGE_CODES`, `ORDER_REJECTION_CODE_RANGE`,
`ORDER_CANCELLED_CODE`. New canonical paths: `ibapi::Notice`,
`ibapi::NoticeCategory`, etc. Add `Notice` and `NoticeCategory` to
`src/prelude.rs` next to the existing `Subscription` re-exports (line 48).

Decision: re-export from `lib.rs`, not from `errors.rs` or a new module.
Smallest diff, no taxonomy debate. `Notice` conceptually overlaps with errors
but is distinct (non-terminal IB warnings vs. terminal `Error`), and the
crate-root home is already where `Client` / `ClientBuilder` / `Error` /
`StartupMessage` live.

**(b) Demote wire internals to `pub(crate)`.**
`RequestMessage`, `ResponseMessage`, `PROTOBUF_MSG_ID`, `encode_length`,
`encode_protobuf_message`, `encode_raw_length`, `encode_request_binary_from_text`,
`order_id_index`, `request_id_index`. All internal callsites use
`crate::messages::*`; visibility narrowing is transparent to them.

**(c) Preserve `record_interactions.rs` reachability.**
The example imports `ibapi::messages::parser_registry::*` and
`ibapi::messages::*` (`examples/record_interactions.rs:12-13`). It uses
`MessageParserRegistry`, `ParsedField`, `IncomingMessages`, `OutgoingMessages`
(verified by grep on the example). After (a), the message-id enums are
reachable from crate root; for `parser_registry`, mark `pub mod messages`
with `#[doc(hidden)]` and keep `parser_registry` `pub` underneath. The
module stays callable (the example still compiles) but is hidden from
docs.rs.

Alternative considered: relocate `parser_registry` to a labelled
`_internal_recording` module. Rejected — adds a name with no downstream
benefit; `#[doc(hidden)]` already signals "unstable, here for tooling."

**Migration guide entry**:

> #### `ibapi::messages` is now opaque
>
> User-facing types (`Notice`, `NoticeCategory`, `IncomingMessages`,
> `OutgoingMessages`, the warning/rejection code-range constants) are now
> re-exported from the crate root and the prelude. Update
> `use ibapi::messages::Notice` → `use ibapi::Notice` (or
> `use ibapi::prelude::*`). Wire-level types (`RequestMessage`,
> `ResponseMessage`, framing helpers) are crate-private in 3.0.

**Self-review before opening:** grep `README.md`, every `docs/*.md`, and
module rustdoc for `ibapi::messages::`; fix or remove every hit (rule 27
sibling — `.md` blocks aren't compile-checked, so verify each manually).

### PR 4 — verification checkpoint (no code expected)

After 1–3 ship:

1. `cargo doc --no-deps` under each feature config — visually confirm no
   `proto::*` or wire-internal types appear on the rendered surface.
2. `rg '^pub (fn|struct|enum|trait|mod|use)' src/ | grep -v test` —
   spot-check ~20 entries: "would a user need this?"
3. Confirm `subscriptions::common::DecoderContext` and `StreamDecoder` are
   unreachable from `ibapi::*` (a future contributor widening the module
   visibility would silently re-leak them).

If audit is clean, this PR doesn't exist. If it surfaces stragglers, ship
the visibility-tightening as a small PR.

## Out of scope

- `pub mod protocol` (`src/lib.rs:127`) — server-version constants; keep `pub`.
  PR 4 confirms nothing plumbing-shaped leaks via `protocol::*`.
- `pub mod trace` — intentional diagnostic API; consumed by user-facing
  `examples/{sync,async}/trace_test.rs`.
- `pub mod display_groups` — domain module.
- A `messages` module split (separating wire framing from notice
  classification from message-id enums) — flagged as a follow-up if review
  asks; not load-bearing for this plan.

## Verification per PR

CLAUDE.md "Quick Commands" applies as-is (three clippy configs, three
rustdoc configs, `just test`, integration-crate builds per rule 11).
