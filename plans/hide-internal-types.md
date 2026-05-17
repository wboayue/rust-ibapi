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
| 1 | `Client::stubbed()` async | `pub` + `#[cfg(test)]` (`src/client/async.rs:342`) | none (gated out of downstream builds) | **shipped PR #574** — `pub(crate)`; matches sync (`src/client/sync.rs:357`) |
| 2 | `Client::message_bus()` async | `pub` + `#[cfg(test)]` (`src/client/async.rs:359`) | none (same gating) | **shipped PR #574** — deleted (zero callers anywhere; internal code reads the field) |
| 3 | `pub mod proto` | `src/lib.rs:130` — generated prost bindings + `proto::{encoders,decoders}` helpers | zero hits in `examples/`, `integration/`, `docs/`, `README.md` | **shipped PR #575** — `pub(crate) mod proto;` + both child modules; deleted dead `decode_error_message` (rule 9) |
| 4 | `pub mod messages` | `src/lib.rs:110` — mixed: a few legitimately-public types, many wire internals | `examples/record_interactions.rs` reaches `parser_registry::*` + the message-id enums | **shipped PR #577** — `pub(crate) mod messages`; user-facing types re-exported from crate root + prelude; `#[doc(hidden)] pub use parser_registry` for the recording example; dead `encode_request_binary_from_text` deleted |
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

### PR 1 — async `stubbed` / `message_bus` → `pub(crate)` ✅ shipped #574

Items #1 and #2 in the audit. `stubbed` narrowed to `pub(crate)`;
`message_bus()` deleted (zero callers; rule 9 "modernize touched module").
No migration guide entry (gated `#[cfg(test)]`, never on the external
surface).

### PR 2 — `pub mod proto` → `pub(crate) mod proto` ✅ shipped #575

Item #3 in the audit. `pub mod proto` and both child modules
(`decoders`, `encoders`) narrowed to `pub(crate)`. Internal callsites
(~200, all via `crate::proto::*`) unaffected. Narrowing surfaced dead
code that prior `pub` visibility had hidden:

- `decode_error_message` — deleted (superseded by the richer
  `decode_error_envelope` in `src/transport/routing.rs:49`, dead since #450).
- Three prost-generated `*End` structs used only in `#[cfg(test)]`
  testdata builders — `dead_code` added to the existing
  `#![allow(missing_docs, clippy::all)]` at `src/proto/mod.rs:1`.

Migration guide §16 added.

### PR 3 — sort and trim `pub mod messages` ✅ shipped #577

Item #4 in the audit. Shipped — `pub mod messages` → `pub(crate)`.

User-facing types re-exported from crate root (split into two
`#[doc(inline)] pub use` blocks for types vs constants):
`Notice`, `NoticeCategory`, `IncomingMessages`, `OutgoingMessages`,
`ResponseMessage`, plus the four code-range constants. `Notice` and
`NoticeCategory` added to the prelude.

`ResponseMessage` stayed `pub` — forced by `StartupMessage::Other(ResponseMessage)`
(`src/connection/common.rs:40`) and `connection::common::parse_raw_message`,
both on the public API surface. Reshaping `StartupMessage::Other` to retire
the leak is genuinely out of scope (see Follow-up below).

Wire internals (`RequestMessage`, `PROTOBUF_MSG_ID`, framing helpers,
message-id index helpers) demoted to `pub(crate)`. `RequestMessage::{encode,
encode_simple, from, from_simple}` gated `#[cfg(all(test, feature = "sync"))]`
on the impl block — all four are sync-only test helpers; the cfg is more
honest than blanket `dead_code` suppression. Dead `encode_request_binary_from_text`
deleted (rule 9).

`parser_registry` reachable via `#[doc(hidden)] pub use crate::messages::parser_registry;`
at the crate root — the crate's first `#[doc(hidden)]` usage, established
as the escape-hatch shape for tooling-only reach-ins.
`examples/record_interactions.rs` updated to use the new crate-root paths.

Migration guide §17 added.

### PR 4 — verification checkpoint ✅ audit clean (2026-05-17)

After 1–3 shipped, ran the three checks. No PR needed.

1. **Rendered docs surface clean** — `cargo doc --no-deps` produces no links
   from `target/doc/ibapi/index.html` to `messages/` or `proto/`. External
   consumers cannot resolve `ibapi::messages::*` or `ibapi::proto::*` —
   verified by building a downstream `test-vis` crate against each: both
   fail with E0603 ("private module"). The `target/doc/ibapi/messages/`
   directory is a rustdoc artifact (rustdoc traverses `pub(crate)` modules
   to locate the canonical definition site of re-exported items); it isn't
   reachable via the public navigation.

2. **`pub` spot-check** — 932 `pub` declarations total. All top-level
   `pub mod` declarations in `src/lib.rs` are intentional (domains +
   `client` + `errors` + `prelude` + `protocol` + `trace`). Two
   over-permissive `pub fn`/`pub struct` declarations inside `pub(crate)`
   modules (`AsyncInternalSubscription` in `transport`, `parse_raw_message`
   in `connection/common`) are crate-private externally and don't leak; a
   `pub` → `pub(crate)` tighten-up on them is a separate cosmetic cleanup
   if anyone cares.

3. **`DecoderContext` / `StreamDecoder` unreachable** — `ibapi::subscriptions::DecoderContext`
   and the suggested fallback `ibapi::subscriptions::common::DecoderContext`
   both fail with E0603 from external code. `subscriptions::common` is
   `pub(crate)`; the `pub(crate) use` re-export at
   `src/subscriptions/mod.rs:5` confines both items to crate scope.

**Correction to PR 3's follow-up entry**: `ResponseMessage` is exposed
externally only via `StartupMessage::Other(ResponseMessage)`
(`src/connection/common.rs:40`). `parse_raw_message` on the same file is
inside `pub(crate) mod connection`, so it's not an external leak. The
follow-up entry's reference to it was wrong and has been pruned.

## Follow-up (out of scope here)

- **Retire `ResponseMessage` from the public surface.** PR 3 had to re-export
  it because `StartupMessage::Other(ResponseMessage)` (`src/connection/common.rs:40`)
  is on the public API surface. Reshape `StartupMessage::Other` to carry a
  typed payload instead of the raw wire envelope — once that lands,
  `ResponseMessage` can become `pub(crate)`.

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
