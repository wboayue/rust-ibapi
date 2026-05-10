# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

The rust-ibapi crate is a Rust implementation of the Interactive Brokers TWS API with both synchronous and asynchronous support.

**Important:** The async client is enabled by default. You can opt into the blocking client with `--features sync`, and the two features may be combined:
- `cargo build` (default features) exposes the async client as `client::Client`
- `cargo build --no-default-features --features sync` enables only the blocking client
- `cargo build --no-default-features --features "sync async"` enables both; the blocking API lives under `client::blocking::Client`

## Documentation Index

### Getting Started
- [**Quick Start Guide**](docs/quick-start.md) - Get up and running in minutes
- [**Examples Guide**](docs/examples.md) - Running and writing examples
- [**Troubleshooting**](docs/troubleshooting.md) - Common issues and solutions

### Core Concepts
- [**Architecture Overview**](docs/architecture.md) - System design, components, and module organization
- [**Feature Flags**](docs/feature-flags.md) - Sync vs async modes and feature guards
- [**API Patterns**](docs/api-patterns.md) - Builder patterns, protocol versions, and common patterns

### Development
- [**Code Style Guidelines**](docs/code-style.md) - Coding standards and conventions
- [**Build and Test**](docs/build-and-test.md) - Build commands, testing patterns, and CI
- [**Testing Patterns**](docs/testing-patterns.md) - Test fixture stratification: `MessageBusStub` / `MemoryStream` / handshake-replay listener
- [**Integration Tests**](docs/integration-tests.md) - Writing tests against a live gateway
- [**Extending the API**](docs/extending-api.md) - Adding new TWS API functionality

## Version 3.0 Philosophy

Version 3.0 is a breaking release. Fix API inconsistencies even when it means breaking changes — consistent naming, idiomatic Rust patterns, and a clean public API take priority over backward compatibility.

## Branches

- **`main`** — 3.x development and releases
- **`v2-stable`** — 2.x maintenance

Changes to both branches should be made via pull requests.

## Key Points to Remember

1. **Feature coverage**: default-async, sync-only, and all-features builds must compile **and** pass tests when touched
2. **Follow module structure**: Client methods live as `impl Client` blocks in domain modules (e.g., `accounts/sync.rs`), not in `client/sync.rs` or `client/async.rs`. Use `common/` for shared logic between sync/async. Protobuf decoders live in each domain's `common/decoders.rs`; shared proto→domain converters live in `proto/decoders.rs`
3. **Run quality checks before committing**: every command in [Quick Commands](#quick-commands) below — formatter, all three clippy configs, all three rustdoc configs. `cargo test --doc` only validates doc-test compilation, not intra-doc link resolution; that's why the rustdoc step is separate
4. **Design principles**: see [docs/code-style.md](docs/code-style.md#design-principles) for SRP, composition, and avoiding repetition. Project-specific constraint: max 3 params per function — use a builder for 4+
5. **Never use `block_on` in async code**: Do not use `futures::executor::block_on()` inside async contexts — it blocks tokio worker threads and risks deadlocks. Use atomics (`AtomicI32`, etc.) for lock-free access to rarely-written values, or make the function `async` and `.await` the lock
6. **Every new function needs a test**: Before opening a PR, verify every new `pub`/`pub(crate)` function has a corresponding unit test. Review test coverage as a final step — missing tests should block the PR. **Overall line coverage target: 90%+** — run `just cover` and check the report; if a touched module drops below 90%, add tests before opening the PR
7. **Pinned Rust toolchain**: `rust-toolchain.toml` pins this branch to a specific Rust version (1.95.0 on `main`, 1.93.0 on `v2-stable`); `.github/workflows/ci.yml` pins `dtolnay/rust-toolchain@<same-version>`. CI and local must agree on the version so clippy lints don't surprise anyone. To upgrade: bump both files in the same PR, fix any new lints, verify CI green
8. **Separate test files**: Always keep tests in their own files, not inline `#[cfg(test)] mod tests` blocks. Prefer flat sibling files (`foo.rs` + `foo_tests.rs`) over a nested module (`foo/mod.rs` + `foo/tests.rs`). The test file declares `use super::*;` and lives next to the implementation. Wire it in with `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;` from the implementation file (or from the parent `mod.rs` for domain submodules)
9. **Modernize touched modules**: When modifying a module for a feature or fix, also bring the rest of it up to current project conventions in the same PR — extract inline tests to a sibling `_tests.rs`, fix small style drift, normalize patterns. Be aggressive: don't leave a module half-migrated. Large mechanical sweeps unrelated to the feature still belong in their own PR
10. **Tests must exercise production code**: Self-loop tests like `builder → encode → decode → assert builder fields` only verify pass-through and prost — they don't catch real bugs. Prefer end-to-end paths: for outgoing requests, drive the real client API and verify captured bytes via `assert_request<B>(builder)`; for incoming responses, feed builder bytes through the production decoder (e.g. `decode_*_proto`). When reviewing a new test, ask "what production code does this traverse?" — if the answer is "none", drop it
11. **Build integration crates when touching wire surfaces**: The `integration/` workspace (`ibapi-integration-sync`, `ibapi-integration-async`, `ibapi-test`) is **not** in `default-members`. Plain `cargo build` / `cargo test` / `cargo clippy --all-targets` skip these crates. When a change touches `Subscription`, the proto encoders/decoders, or anything wire-format-adjacent, also run `cargo build -p ibapi-integration-sync --tests` and the async equivalent (and the matching `cargo clippy -p ... --tests -- -D warnings`). Compilation against these crates is the contract — no live gateway needed for this check
12. **Dual-feature public types**: When adding a public type with distinct sync/async impls (different receiver types, different async-ness), follow the `Subscription` / `NoticeStream` dual-export pattern — separate per-feature submodules (`sync_impl`, `async_impl`), top-level alias prefers async (`#[cfg(feature = "async")] pub use ...; #[cfg(all(feature = "sync", not(feature = "async")))] pub use ...`), sync version also exported at `client::blocking::*`. Trait method return types and `Client::*` API methods must spell out the per-feature path (`crate::subscriptions::foo::sync_impl::Foo`), not the top-level alias. Naive same-name sibling structs gated `#[cfg(feature = "sync")]` / `#[cfg(feature = "async")]` collide under `--all-features`; always run `cargo check --all-features` to catch this
13. **Skip wrappers when async runtime provides the abstraction**: Don't wrap `tokio::sync::broadcast::Sender` / `mpsc` / etc. in a struct just for sync/async parity. The sync side often genuinely needs the wrapper (e.g. `Mutex<Vec<crossbeam Sender>>` + manual prune); the async side often has subscribe + auto-prune built-in, so a wrapper would be a no-op delegate. Store the runtime type directly on the bus. Acceptable mirror duplication (`filter_data` / `filter_data_stream`, `deliver_to_request_id` impls) is the precedent — both sides have real behavior; pure parity-wrappers are not
14. **Narrow re-exports over widened module visibility**: To expose 1–3 items from an otherwise-private module, prefer `pub(crate) use foo::{bar, baz};` at the parent over widening the module declaration to `pub(crate) mod foo`. The latter exposes every `pub(crate)` item inside the module to the whole crate; the former exposes exactly the names you intend. Common trigger: cross-domain code (e.g. `connection/` reaching into `orders/` or `accounts/` decoders) — keep that cross-cut narrow
15. **Decoder dispatch + skip-classified catch-all**: Any decoder that reads fields off a `ResponseMessage` must dispatch via `message.decode_proto_or_text(proto, text)` (or `decode_proto_or_text_owned` if the text path takes ownership). When `server_version >= PROTOBUF` (201), TWS sends most messages in protobuf form with `fields = [msg_id_str]` and the payload in `raw_bytes`; running the text decoder on that EOFs on field 2. The catch-all arm in `impl StreamDecoder<T>::decode` must be `_ => Err(Error::UnexpectedResponse(message.clone()))` — `process_decode_result` skip-classifies that variant; `Err(Error::NotImplemented)` and `Err(Error::Simple(...))` terminate the subscription on any unknown message type. Sister conversions (`From<&ResponseMessage> for Notice`, `From<ResponseMessage> for Error`) are already proto-aware in this codebase — prefer them over hand-rolling `is_protobuf` branches at decoder call sites. Issue #508 was the bug class
16. **Typing previously stringly-typed fields**: For v3.0 String→enum migrations (e.g. PR #518's `OrderStatus.status: OrderStatusKind`), follow the `Action`/`OrderStatusKind` precedent — strict enum, `Display` round-trips back to the IB wire string, `FromStr` returns `Result<_, Error>` (not panicking via `todo!()` like older `Action::from(...)`). **Decoder must reject empty/missing inputs as `Error::Parse`, not fall back to `T::default()`** — silent default masks incomplete TWS responses and lets monitoring loops hang. Helper shape: `fn parse_X(opt: &Option<String>) -> Result<X, Error>` next to the existing `parse_f64` siblings in `proto/decoders.rs`. Before typing a `String` field as enum, **verify the wire actually carries enumerated values** — grep captured-wire fixtures (e.g. `decoders/tests.rs`) and the C# reference at `/Users/wboayue/projects/tws-api/source/csharpclient/client/`; field-name resemblance to a known vocabulary is misleading (the `OrderState.completed_status` case: name suggests enum, wire is free-form `"Cancelled by Trader"`). Also add a unit test asserting both `None` and `Some("")` produce `Err(Error::Parse(..))`, and a unit test of the `Display`/`FromStr` round-trip table
17. **`ResponseMessage` accessors must be proto-aware (sister to rule 15)**: Any `&self` accessor on `ResponseMessage` that reads by text-field index — `peek_int`/`peek_string`/`request_id`/`order_id`/`execution_id` and any future siblings — needs an `is_protobuf` branch. For `server_version >= PROTOBUF`, `fields = [msg_id]` and the payload lives in `raw_bytes`; the text-index path silently fails (returns `None`, breaking subscription routing) or panics (`peek_string` was the bug-class root in PR #519). Use the `proto_or_text_int` / `proto_or_text_string` helpers in `messages.rs` as the unifying shape. Cursor primitives (`peek_*`) all return `Result<T, Error>`; never reintroduce a panicking variant. **Don't decode the full proto struct just to read one field** — define a minimal `prost::Message` envelope (e.g. `ProtoIdEnvelope { id @ tag 1 }`, `ExecutionDetailsMinimal { execution @ tag 3 → order_id @ 1, exec_id @ 2 }`) and let prost length-skip the rest. The dispatcher calls these accessors 3–4× per inbound message; full decodes of `OpenOrder` / `ExecutionDetails` cost ~20 String allocations each, minimal envelopes are essentially free. The proto bug-class family now spans decoders (rule 15), `From<*ResponseMessage>` conversions, and these accessors — when fixing one, audit the other two
18. **Public API needs a doc-example**: every `pub fn` / `pub` constructor / public builder entry point gets a `# Examples` block with a runnable (`no_run` / `ignore` is fine) example showing the canonical happy-path call. The example is part of the contract — it teaches the idiom, doubles as a compile-time regression guard against signature drift, and matches what users see on docs.rs. Don't drop it as "redundant with the builder's `subscribe()` example"; the entry point and the terminal action are different surfaces. Tiny accessors (struct field getters, trivial `is_*` predicates) are exempt — examples on those would be noise
19. **Sweep test fixtures when a decoder goes proto-only**: After deleting a text branch in a domain decoder (workflow: `plans/legacy-text-protocol-cleanup.md`; precedents: PRs #529 / #531 / #532 / #534 / #543), find every `MessageBusStub` fixture feeding text-framed responses for that message type — `text_response(builder.encode_pipe())` in `<domain>/{sync,async}/tests.rs` — and convert to `proto_response(IncomingMessages::Foo, builder.encode_proto())`. Silent skip-classification (per rule 15) means a missed conversion shows as the test passing but post-`next_data()` assertions never running; verify with sync + async + all-features sweeps. Prereq: `ResponseProtoEncoder::encode_proto()` on the builder. **Field-minimal builders are the standard** even for deeply-nested protos — PR #534's `ContractDataResponse` covers ~50 `proto::ContractData` fields with ~15 setters by working backwards from test validators; document load-bearing `Default` values (e.g. `min_size: "1"` because validators assert `min_size == 1.0`). **Builders live in `src/testdata/builders/<domain>.rs`** alongside the existing news/scanner/orders/positions builders — never roll a sibling `test_helpers.rs` under `<domain>/common/` for the same purpose; PR #543 /simplify caught exactly this misplacement (free positional-arg helpers + `#[allow(clippy::too_many_arguments)]` ⇒ refactored to `RealTimeBarTickResponse` etc. in `testdata/builders/market_data.rs`). `#[allow(clippy::too_many_arguments)]` on a fixture-construction helper is the canary, not the fix — see rule 4. Split the builder PR from the test-migration PR only when the test consumer needs it, not for proto depth alone. For table-driven fixtures: rename `response_messages: Vec<String>` → `ordered_responses: Vec<ResponseMessage>` on the test case struct, use `proto_response(...)` for migrated message types and `text_response(...)` for unmigrated end markers / errors / cross-domain shared decoders
20. **Ratchet PRs and decoder cleanups split**: For the legacy-text-protocol-cleanup work, ship the floor ratchet (`require_protobuf_support` constant bump + test-fixture version sync) and the per-family decoder text-branch deletion as separate PRs (precedent: #527 → #529, #530 → #531). The bump is mechanical; each cleanup needs C# `EDecoder.cs` verification that the family's proto/text dispatch isn't server-version-gated *within* the case (it's purely on the 4-byte msg-id framing, no `if server_version >=` guards). Multi-gate ratchets (e.g. 203 → 210 skipping 6 gates) are safe IFF every family in the skipped range already has a proto decoder + `decode_proto_or_text` wrapper in place — verify against the per-family inventory table in the plan file before bumping
21. **Derive test expectations from the constant under test**: When a test asserts against version-gated APIs (`Features::*.min_version`, `Features::*.name`, `server_versions::*`, `IncomingMessages::*`), bind the constant once and derive every assertion from its fields — never hardcode the wire value. ❌ `assert_eq!(required, 137); assert_eq!(name, "tick-by-tick data")` — ✅ `let feature = Features::TICK_BY_TICK; assert_eq!(*required, feature.min_version); assert_eq!(name, feature.name)`. Boundary cases become self-documenting (`feature.min_version - 1`, `feature.min_version`). For Display assertions, build the expected string with `format!()` against the same fields — still validates ordering and literal punctuation, but parameterizes the values. **Why this matters**: hardcoded numbers silently decay when an upstream constant advances (IBKR bumps `MIN_SERVER_VER_*`, the test still passes syntactically but no longer asserts what its name claims). /simplify caught 3 such sites in PR #540
22. **Pin `compile_fail` doc-tests to error codes**: A bare `compile_fail` annotation passes for *any* compilation failure — a renamed import, a missing trait, a future rustc diagnostic change. Pin to the specific code (`compile_fail,E0639`, `compile_fail,E0658`, etc.) so a regression in the *guarded* behavior surfaces instead of silently passing for the wrong reason. Precedent: PR #548's `Contract` lockdown doc-test pins `compile_fail,E0639` (rustc's "cannot construct `#[non_exhaustive]` struct externally"). Find the code by removing the annotation, running the snippet through `rustc`, reading the `error[ENNNN]` line. Same logic applies to `trybuild` `compile_fail` files
23. **Restrictive API additions: modernize callers first, restrict second**: For PRs that add compile-time restrictions to a public type — `#[non_exhaustive]`, removed `Default` impls, removed/renamed `pub` fields, `#[must_use]` on a builder — split into PR-A "modernize callers to a workspace-green alternative" + PR-B "add the restriction." Each PR keeps the workspace green; the restriction PR stays small and reviewable once callers are out of the way. Precedent: PR #547 modernized 15 example sites + a README snippet → PR #548 added `#[non_exhaustive]` on `Contract` + the parity regression test. Distinct from rule 20 (which is specifically about proto floor ratchets); same shape, different domain

## Quick Commands

```bash
# Format code
cargo fmt

# Run clippy (cover every configuration)
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features

# Check rustdoc intra-doc links (separate from `cargo test --doc`)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Run all tests
just test

# Build integration crates (compile check, no live gateway)
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests

# Generate coverage report (opens HTML report in browser)
just cover
```

## Connection Settings

When running examples or tests:
- **IB Gateway Paper Trading**: 127.0.0.1:4002 (recommended)
- **IB Gateway Live Trading**: 127.0.0.1:4001
- **TWS Paper Trading**: 127.0.0.1:7497
- **TWS Live Trading**: 127.0.0.1:7496

## Environment Variables

```bash
# Set log level
RUST_LOG=debug cargo run --example <example_name>

# Record TWS messages for debugging
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --example <example_name>
```

## Git Commit Guidelines

- DO NOT include "Generated with Claude Code" or similar attribution in commit messages
- Keep commit messages focused on the technical changes and their purpose

## Release Notes Guidelines

Use this format for GitHub release notes:

- Group changes under `## What's New` and `## Bug Fixes` headings as applicable
- Each item gets an `### H3 heading` with short description and PR number (e.g., `### Feature name (#123)`)
- One-sentence summary below the heading
- A code sample showing typical usage in a fenced ```rust block
- Order items by significance (most impactful first)

## Maintaining Documentation

Keep `CLAUDE.md`, `README.md`, and documentation up to date as the codebase evolves. When patterns change, conventions are established, or new modules are added, update the relevant files.

### Keep `README.md` and `docs/migration-3.0.md` in sync with v3.0 work

Treat `README.md` and `docs/migration-3.0.md` as part of the public API. Every PR that lands a v3.0 breaking change must update both in the same PR — leaving them stale produces the worst kind of drift, where the migration guide tells users to follow patterns that no longer compile.

Update `docs/migration-3.0.md` whenever the PR:

- Removes or renames a public type, struct field, enum variant, method, or re-export.
- Changes the type of a public field (`String` → typed enum, `bool` → typed mode enum, etc.).
- Changes the shape of a return type (e.g. `Subscription<T>::next()` envelope changes, new `Result` variants).
- Adds or removes a public builder method, callback hook, or feature flag that 2.x users would discover via search.

Update `README.md` whenever the PR:

- Touches code shown in any README example (the examples must still compile and reflect the canonical idiom).
- Removes a variant matched on in any README `match` block.
- Adds an idiom that should be the canonical happy-path (e.g. `is_terminal()` instead of magic-string compares — once shipped, the README should show the new form).

Mechanical check before opening the PR: grep `README.md`, every `docs/*.md`, and module-level rustdoc for any name you changed, removed, or replaced in this PR. Stale references are blockers, not nits.

**Markdown fenced code blocks aren't compile-checked.** `cargo test --doc` only runs `# Examples` blocks in `.rs` files; ```rust blocks in `README.md` and `docs/*.md` are prose. They rot silently every time a field is renamed, a method removed, or a public type reshaped — and there's no CI gate to catch it. After grepping, *read each remaining hit* and verify the snippet would compile against current public API (mental compile pass: do those identifiers exist? do those methods chain on those receivers? are field types still spelled that way?). PR #549's order-construction sweep surfaced six broken `order_builder::market_order(...).condition(...).build()` chains in `docs/api-patterns.md` (Order has no `.condition()` method) and `Order { lmt_price: ..., tif: "GTC".to_string() }` blocks in `docs/order-types.md` (real fields: `limit_price`, `tif: TimeInForce`) — both shipped wrong for months because nothing tested them.

Cross-link in both directions: a new section in `docs/migration-3.0.md` should usually be linkable from a README example or its surrounding prose, and the README's "Migrating?" pointer near the top should keep working.
