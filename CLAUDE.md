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
- [**Rule Nodes**](docs/rules/README.md) - Project conventions as linked nodes; indexed below under [Rule index](#rule-index)
- [**Code Style Guidelines**](docs/code-style.md) - Coding standards and conventions
- [**Build and Test**](docs/build-and-test.md) - Build commands, testing patterns, and CI
- [**Testing Patterns**](docs/testing-patterns.md) - Test fixture stratification: `MessageBusStub` / `MemoryStream` / handshake-replay listener
- [**Integration Tests**](docs/integration-tests.md) - Writing tests against a live gateway
- [**Extending the API**](docs/extending-api.md) - Adding new TWS API functionality

## Version 3.0 Philosophy

Version 3.0 is a breaking release. Fix API inconsistencies even when it means breaking changes — consistent naming, idiomatic Rust patterns, and a clean public API take priority over backward compatibility.

## Branches

- **`main`** — 3.x development and releases. **This is the only actively maintained branch.**
- **`v2-stable`** — 2.x maintenance, frozen. Only touch it when a **specific bug** must be backported there, and only when explicitly asked.

Default all work — features and fixes — to `main` alone. Do **not** open v2-stable PRs by default; only backport a specific named bug on explicit request. Changes to either branch go through pull requests.

## Rule index

Detail lives in [`docs/rules/`](docs/rules/README.md), one directive per file. Follow the
link when the situation matches — the node carries the mechanics, the bug class, and the
precedents. Everything not yet migrated is still inline under Key Points below.

### Wire protocol — protobuf-only at `server_versions::PROTOBUF_REST_MESSAGES_3` (213)

- **Writing or modifying a domain decoder** → [proto-only decoding](docs/rules/wire/proto-only-decoding.md)
- **Adding a `ResponseMessage` accessor, or a public API on a proto inbound message type** →
  [proto-aware accessors](docs/rules/wire/proto-aware-accessors.md)
- **Typing a `String` field as an enum** → [wire enum typing](docs/rules/wire/enum-typing.md)

### Testing

- **Adding a `pub` / `pub(crate)` fn, or checking coverage before opening a PR** →
  [coverage floor](docs/rules/testing/coverage-floor.md)
- **Reviewing a new test, or asserting on captured request bytes** →
  [exercise production code](docs/rules/testing/exercise-production-code.md)
- **Building a response test fixture** → [fixture builders](docs/rules/testing/fixture-builders.md)
- **Writing a `#[cfg(test)] mod tests` block** → [sibling test files](docs/rules/testing/sibling-test-files.md)
- **Asserting against a version-gated API** → [derive from constants](docs/rules/testing/derive-from-constants.md)
- **Writing a doc-test that must *not* compile** → [pin compile_fail codes](docs/rules/testing/pin-compile-fail-codes.md)
- **A function that reads the clock and then branches** → [clock seams](docs/rules/testing/clock-seams.md)

### Sync / async parity

- **Touching feature-gated code, or adding a `lib.rs` doctest** →
  [feature matrix](docs/rules/parity/feature-matrix.md) — note `--features sync` is *not* the
  sync-only build, and no PR gate covers sync-only
- **Reaching for `block_on`, or needing a lock inside an `async fn`** →
  [no block_on](docs/rules/parity/no-block-on.md)
- **Adding a public type with distinct sync/async impls** →
  [dual-feature types](docs/rules/parity/dual-feature-types.md)
- **Wrapping a tokio channel so both sides look alike** →
  [no parity wrappers](docs/rules/parity/no-parity-wrappers.md)
- **Consuming an async `Subscription<T>`** →
  [subscription consumer idiom](docs/rules/parity/subscription-consumer-idiom.md)

### Workflow

- **About to commit or open a PR** → [pre-PR checks](docs/rules/workflow/pre-pr-checks.md) —
  the rustdoc trio is a local-only gate; CI does not fail on broken intra-doc links
- **Touching `Subscription`, a proto encoder/decoder, or any public API shape** →
  [integration crate builds](docs/rules/workflow/integration-crate-builds.md) — those crates are
  outside `default-members`, so every automated gate skips them
- **Editing a module that still carries old idioms** →
  [modernize touched modules](docs/rules/workflow/modernize-touched-modules.md)
- **Adding `#[non_exhaustive]` / `#[must_use]`, removing a `pub` field, or narrowing to
  `pub(crate)`** → [restrict after callers](docs/rules/workflow/restrict-after-callers.md)
- **A clippy lint fires locally but not in CI, or you are upgrading Rust** →
  [pinned toolchain](docs/rules/workflow/pinned-toolchain.md)

> **Two traps that pass CI silently.** A new public API on a proto inbound message type needs
> a `text_request_id_field` entry in `src/messages.rs` — `MessageBusStub` tests sit below the
> dispatcher and pass without it; see
> [proto-aware accessors](docs/rules/wire/proto-aware-accessors.md). And a text-framed fixture
> reaching a proto-only decoder is skip-classified, so the test goes green with its
> post-`next_data()` assertions unrun; see
> [fixture builders](docs/rules/testing/fixture-builders.md) and
> [proto-only decoding](docs/rules/wire/proto-only-decoding.md). Neither failure announces
> itself; read the linked nodes before touching either surface.

Rule numbers 1, 3, 5–13, 15–17, 19–24, and 26 are retired, not reused — the gaps are
deliberate.
Numbering is dropped entirely once the remaining clusters migrate; see
[plans/claude-md-knowledge-graph.md](plans/claude-md-knowledge-graph.md).

## Key Points to Remember

2. **Follow module structure**: Client methods live as `impl Client` blocks in domain modules (e.g., `news/sync.rs`), not in `client/sync.rs` or `client/async.rs`. Prefer the flat `<domain>/sync.rs` + `<domain>/async.rs` layout over `<domain>/sync/mod.rs` — the project minimizes `mod.rs` files. Use `common/` for shared logic between sync/async. Protobuf decoders live in each domain's `common/decoders.rs`; shared proto→domain converters live in `proto/decoders.rs`
4. **Design principles**: see [docs/code-style.md](docs/code-style.md#design-principles) for SRP, composition, and avoiding repetition. Project-specific constraint: max 3 params per function — use a builder for 4+. The receiver (`self` / `&self` / `&mut self`) does not count toward this budget; `pub fn foo(&self, a, b, c)` is at the limit, `pub fn foo(&self, a, b, c, d)` needs a builder. **Builder rationale matters**: the rule's goal is to spare callers from positional-argument noise when there are optional / defaultable fields. A 4+ param function where *every* arg is required with no reasonable default gains little from a builder — `client.foo(a, b, c, d, e)` is no worse than `client.foo(a).b(b).c(c).d(d).e(e).run()`. For all-required signatures, prefer grouping related args into a struct (e.g. `DateRange { start, end }`) or accept the violation with a comment, rather than mechanical builder conversion
14. **Narrow re-exports over widened module visibility**: To expose 1–3 items from an otherwise-private module, prefer `pub(crate) use foo::{bar, baz};` at the parent over widening the module declaration to `pub(crate) mod foo`. The latter exposes every `pub(crate)` item inside the module to the whole crate; the former exposes exactly the names you intend. Common trigger: cross-domain code (e.g. `connection/` reaching into `orders/` or `accounts/` decoders) — keep that cross-cut narrow
18. **Public API needs a doc-example**: every `pub fn` / `pub` constructor / public builder entry point gets a `# Examples` block with a runnable (`no_run` / `ignore` is fine) example showing the canonical happy-path call. The example is part of the contract — it teaches the idiom, doubles as a compile-time regression guard against signature drift, and matches what users see on docs.rs. Don't drop it as "redundant with the builder's `subscribe()` example"; the entry point and the terminal action are different surfaces. Tiny accessors (struct field getters, trivial `is_*` predicates) are exempt — examples on those would be noise
25. **Macros only when ordinary Rust can't express the pattern**: Macros add a translation layer — readers parse syntax twice, error spans drift, and tooling (goto-def, rust-analyzer expansion) degrades. Default to generics, trait impls, helper functions, slices + `for`-loops, or table-driven tests with parameterized inputs. **Per-type `#[test]` granularity alone is not enough** — a generic `fn check_X<T: ...>(sample: &str)` + thin per-type `#[test] fn` wrappers gives the same independent pass/fail and clear test names, without the macro tax. Macros earn their cost in three specific cases: (a) **shape-identical impls across newtypes/wrappers** that can't be deduplicated via a blanket trait impl — precedent: PR #548's `impl_str_partial_eq!` collapsed 12 `PartialEq<str>` impls (4 directions × 3 newtypes) into 3 invocations, −55 lines; (b) **bodies that rely on inherent methods or constructs that don't compose generically** — `<$t>::new`, `.as_str()`, struct-literal access without a trait, etc. — where defining a test-only trait costs more than the macro saves. Precedent: `string_newtype_surface!` in `src/contracts/types_tests.rs` calls inherent `<$t>::new` and `.as_str()`. **Counter-examples**: (i) a serde round-trip uses only trait methods (`Serialize`/`Deserialize`/`From<&str>`) — PR #554 originally wrote `serde_round_trip!`, then refactored to a generic `check_serde_round_trip<T>` + 3 thin `#[test] fn` wrappers; (ii) the `str_eq_round_trip!` macro (originally PR #548) was demoted in PR #554 to `check_str_partial_eq_round_trip<T>` — the `PartialEq<str>` / `PartialEq<&str>` impls are traits, so a `where T: PartialEq<str> + for<'a> PartialEq<&'a str> + for<'a> From<&'a str>, str: PartialEq<T>, for<'a> &'a str: PartialEq<T>` clause works, even if the HRTB bounds look heavy; (c) **contexts where generics aren't legal** — `const` initializers, `prost::Message` derive bodies, declarative DSL surfaces. Before writing a `macro_rules!`, ask: would a generic function with a trait bound work? a `for case in [(x, y), ...]` loop? a table-driven `#[rstest]`-style test? If yes, take that path. **/simplify must challenge new macros twice**: first whether they can collapse together (PR #554 folded `string_newtype_new_monomorphizations!` into `string_newtype_surface!`), then whether each surviving macro's body actually needs the macro (PR #554 demoted `serde_round_trip!` to a generic helper after the first /simplify pass missed it). Sister to rule 4 (composition over repetition) and the `Macro out repeated trait impls` memory; the inverse of "speculative macro infrastructure". **Crate-wide macro home**: shape-identical trait-impl macros that cross module boundaries live in `src/macros.rs`, reachable via `#[macro_use] mod macros;` in `lib.rs`. Currently hosts `impl_str_partial_eq!` (for string-newtype `PartialEq<str>` impls) and `impl_wire_enum!` (for `Display` / `FromStr<Err = Error>` / `ToField` from `as_str` + `from_wire` data tables). When promoting a module-local macro crate-wide, copy it to `src/macros.rs`; don't `#[macro_export]` (that leaks to the public API on docs.rs)
27. **"Doc parity" requests imply a signature audit**: When the ask is "make async docs match sync" (or vice versa) on a domain module, audit **three** layers before opening the PR, not just the asked-for one: (a) **doc content** — `# Arguments`, `# Examples` blocks, struct-level docs (the asked-for thing); (b) **parameter naming** — same-semantics-different-name drift like `interval_end` vs `end_date`; (c) **signature shape** — `Option<T>` vs `T`, separate methods vs single-with-`Option`-arg, terminal-types vs typed-args. The doc gap is the trigger; signature divergence is often the bigger fish. Per [modernize touched modules](docs/rules/workflow/modernize-touched-modules.md), fix all three in the same PR. Precedent: PR #573 (issue #210) — original ask was "doc parity" for async historical; what shipped was 9 docs + 1 signature reshape (`Option<WhatToShow>` → `WhatToShow` on `historical_data` + `historical_data_streaming`) + 1 method split (`historical_schedule` → `historical_schedules` + `historical_schedules_ending_now`) + 1 parameter rename (`interval_end` → `end_date`). Sister rules: [modernize touched modules](docs/rules/workflow/modernize-touched-modules.md), [dual-feature types](docs/rules/parity/dual-feature-types.md) (the *types* axis); this rule covers the docs/naming/shape axis of dual-feature parity. **Sub-rule for `Option<T>` removal**: before flipping `Option<T>` → `T` on a public arg, run the three-source check — encoder/wire (is `None` representable?), C# `EClient.cs` (is field documented as required?), all callers (what fraction wrap with `Some(...)`?). If wire allows but every real caller wraps, drop the `Option`. Watch out: tests passing `None` for incidental laziness (exercising an early-failure path where the arg is irrelevant) don't count as "real `None` callsites." **Sub-rule for "magic-`None` splits"**: when a public `foo(arg: Option<T>, ...)` API surfaces magic-`None` problems and gets split into `foo(arg: T)` + `foo_default(...)` — accept the split as the **correct intermediate** in a 3-step evolution: bad shape (magic-`None`) → split → fluent builder with explicit `.named_setter(value)` for the optional. The destination is the builder; the split is a planned waypoint, not a final answer. **Per-method cross-feature pairing for doc-examples**: when async doc-example imports differ from sibling async methods in the same file, prefer matching the sync counterpart per-method (intentional pairing) over intra-file uniformity — /simplify reviewers who flag "intra-file inconsistency" on dual-feature docs are often missing the pairing axis (PR #573 /simplify deferred the `prelude`-import streaming example for this reason)

## Quick Commands

The pre-PR gate, in full — see
[pre-PR checks](docs/rules/workflow/pre-pr-checks.md) for what each one catches and which of
them CI does *not* run:

```bash
# Format code
cargo fmt

# Clippy, one run per feature configuration.
# `--features sync` alone keeps the default async client on; use
# --no-default-features for the genuine sync-only build.
# See docs/rules/parity/feature-matrix.md.
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features sync -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings

# Rustdoc intra-doc links — local-only gate; `cargo test --doc` misses these
# and CI's `cargo doc` runs without RUSTDOCFLAGS.
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Run all tests (one leg per feature configuration)
just test

# Examples are a caller surface `just test` never compiles
cargo build --examples
cargo build --examples --no-default-features --features sync
```

Situational — run when the change touches the matching surface:

```bash
# Wire surfaces: integration crates are outside default-members, so every
# other gate skips them. See docs/rules/workflow/integration-crate-builds.md.
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests

# CLAUDE.md, docs/rules/, or plans/ — validates the rule graph and its index
just rules-check

# Coverage report, nightly-only. See docs/rules/testing/coverage-floor.md.
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

## Changelog

Maintain a root `CHANGELOG.md` in [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) format, versioned per [SemVer](https://semver.org/spec/v2.0.0.html).

- **Every PR with a user-facing change adds an entry under `## [Unreleased]`** in the same PR, grouped by change type — `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` (omit empty groups, keep them in that order). A stale changelog is a blocker, same as a stale README/migration guide.
- Internal-only work (refactors, tests, CI, doc-only edits, dependency bumps with no behavior change) needs **no** entry. If unsure, ask "would a downstream user notice?" — if no, skip it.
- One bullet per change: imperative, concise, ending with the PR number — e.g. `- Classify TWS codes 10089/10167 as informational so delayed-data subscriptions stay open (#677).` Breaking changes go under `Changed`/`Removed` and must also be reflected in `docs/migration-3.0.md`.
- **On release**: rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (ISO 8601 date), then add a fresh empty `## [Unreleased]` at the top. Keep version sections newest-first. Update the link-reference definitions at the bottom (`[Unreleased]` → `compare/vX.Y.Z...HEAD`, `[X.Y.Z]` → the tag/compare URL).
- The changelog is the curated, append-as-you-go history; GitHub release notes (above) are the same entries expanded with code samples at release time. Keep the two consistent.

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
