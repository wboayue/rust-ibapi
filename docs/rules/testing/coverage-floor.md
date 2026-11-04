---
id: coverage-floor
title: Every new function needs a test; 90% line-coverage floor
cluster: testing
status: active
triggers:
  - adding a pub or pub(crate) function
  - final review pass before opening a PR
  - a coverage report reads 0% for every file
  - a const fn shows no coverage despite many call sites
symbols: [ProtocolFeature::new, cargo-llvm-cov, just-cover]
related: [exercise-production-code, clock-seams, fixture-builders]
precedents: ["#540", "#554"]
---

Every new `pub` / `pub(crate)` function gets a unit test. Check this as a final step before
opening the PR — a missing test is a blocker, not a nit.

**Line-coverage floor is 90%.** Run `just cover`; if a module you touched sits below it, add
tests before opening the PR.

## Why

`just cover` runs `cargo +nightly llvm-cov --all-features --doctests --html --open`. Nightly is
required to instrument doc-tests (rustdoc's `--persist-doctests` is unstable). Stable with
`--lib` misses every doc-test and reports phantom uncovered regions on `..Default::default()`
and doc-comment fence lines. Nightly is for coverage only (`just cover` and
`.github/workflows/coverage.yml`); build and test stay on the pinned stable toolchain.

Two results that are not coverage numbers:

- **0% for every file is a tooling failure** — `cargo-llvm-cov` is older than the nightly.
  Cross-check with stable `cargo llvm-cov --lib --all-features`.
- **A `const fn` called only from `const` initializers reads 0%.** llvm-cov sees runtime paths
  only. One trivial runtime test fixes it.

A percentage means nothing without its baseline: before claiming a module regressed, measure
main too (`src/proto/decoders.rs` once read 85% on a branch against 41.7% on main).

For a single module, a `--no-report` run plus `jq` over the JSON report takes about a minute;
`just cover` renders everything. The loop, the baseline step and the tactics for the last gap
are in the `/coverage` skill ([`.claude/skills/coverage`](../../../.claude/skills/coverage/SKILL.md)).

## Precedents

- #540 — `ProtocolFeature::new` reported 0% despite every `Features::*` constant calling it;
  `protocol_feature_new_constructs_runtime_value` in `src/protocol_tests.rs` is the fix.
- #554 — `contracts/types.rs` 61.4% → 99.6%, via [clock seams](clock-seams.md) plus serde
  round-trips and both `&str` and `String` monomorphizations of `impl Into<T>`.
