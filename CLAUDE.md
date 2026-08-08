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
precedents. Every rule now lives in a node; nothing is inline.

### Wire protocol — protobuf-only at `server_versions::PROTOBUF_REST_MESSAGES_3` (213)

- **Writing or modifying a domain decoder** → [proto-only decoding](docs/rules/wire/proto-only-decoding.md)
- **Adding a `ResponseMessage` accessor, or a public API on a proto inbound message type** →
  [proto-aware accessors](docs/rules/wire/proto-aware-accessors.md)
- **Typing a `String` field as an enum** → [wire enum typing](docs/rules/wire/enum-typing.md)

### Code structure and style

- **Adding a client method, or creating a domain module** →
  [domain module layout](docs/rules/style/domain-module-layout.md)
- **Writing a function that takes four or more parameters** →
  [param budget](docs/rules/style/param-budget.md) — clippy's `too_many_arguments` only fires
  at eight, so nothing gates this
- **Exposing one or two items from an otherwise-private module** →
  [narrow re-exports](docs/rules/style/narrow-reexports.md)
- **About to write a `macro_rules!`, or reviewing one** →
  [macros last resort](docs/rules/style/macros-last-resort.md)

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

### Documentation

- **Adding a `pub fn`, constructor, or builder entry point** →
  [public API examples](docs/rules/docs/public-api-examples.md)
- **Asked to make async docs match sync, or removing an `Option<T>` argument** →
  [doc parity audit](docs/rules/docs/doc-parity-audit.md) — the doc gap is the trigger, the
  signature drift underneath is the job
- **Opening a PR that changes public behavior** →
  [changelog entry](docs/rules/docs/changelog-entry.md) — same PR, under `## [Unreleased]`
- **Removing or renaming anything public** →
  [user docs sync](docs/rules/docs/user-docs-sync.md) — `README.md` and `docs/migration-3.0.md`
  ship with the change; their fenced Rust snippets are compiled by nothing
- **Drafting GitHub release notes** → [release notes](docs/rules/docs/release-notes.md)
- **Establishing, changing, or retiring a project convention** →
  [maintaining the rule graph](#maintaining-the-rule-graph), below

Rule numbers are retired. All 27 are nodes now, addressed by name; a "rule N" citation found in
an old comment, plan, or memory has to be resolved against the `CLAUDE.md` of its own date, not
against this file — the numbering shifted at least once while it was in use. See
[plans/claude-md-knowledge-graph.md](plans/claude-md-knowledge-graph.md).

## Maintaining the rule graph

Nodes ship in the PR that makes their claim true, not afterwards. Mechanics — frontmatter,
trigger phrasing, clusters, `status` — are in the [rules README](docs/rules/README.md). These
have to fire without opening it, because **`just rules-check` validates structure only** (links,
ids, `related`, no `@`-imports, no `file.rs:NNN`) and cannot tell whether a node is still true.

- **Establishing, changing, or retiring a convention** → it gets a node and one index line
  above, or it is not a convention. Changed: rewrite the directive, extend `precedents`.
  Retired: `status: historical`, drop the index line, never delete the node — a concluded arc
  has no trigger, but its reasoning is the expensive part.
- **Deleting or renaming anything a node might cite** → `grep -rn <symbol> docs/rules/ plans/`
  in the same PR. A node describing a function that no longer exists reads exactly like one
  that is correct.
- **Writing a count or completeness claim** — "all N sites", "fully applied", "zero remaining",
  "every `pub fn`" → run the command that produces it as you write it, and leave the command in
  the node. Checking that a cited symbol exists does not check a count, and this is the claim
  class that has been wrong most often — twice in prose warning about invented counts.
- **Landing a PR that proves, extends, or contradicts a node** → append to `precedents:` with
  one line on how it ended. One that became a counter-example is worth more than one that
  confirmed the rule; record it as such rather than dropping it.
- **Reading a node that calls its own failure mode silent, unenforced, or ungated** → that is a
  missing gate, not a documentation problem. Make the failure loud; then rewrite the node's
  claim rather than appending to it, and delete regression tests the gate made impossible.
- **Touching `CLAUDE.md`, `docs/rules/`, or `plans/`** → `just rules-check` before the PR.

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

