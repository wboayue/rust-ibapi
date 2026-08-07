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
memory: [reference_llvm_cov_toolchain_pairing, feedback_llvm_cov_targeted_workflow, feedback_const_fn_coverage_gotcha, reference_llvm_cov_lines_metric, feedback_coverage_mop_up_tactics]
---

Every new `pub` / `pub(crate)` function gets a unit test. Verify this as a final step before
opening the PR — a missing test is a blocker, not a nit.

**Line-coverage floor is 90%.** Run `just cover`; if a module you touched sits below it, add
tests before opening the PR.

## Why

`just cover` is `cargo +nightly llvm-cov --all-features --doctests --html --open`. Nightly is
required because rustdoc's `--persist-doctests` — the hook llvm-cov needs to instrument
doc-tests — sits behind `-Z unstable-options`. Stable with `--lib` alone both misses every
doc-test *and* inserts phantom uncovered regions on `..Default::default()` and `/// ```` lines,
so a file like `contracts/mod.rs` shows roughly 12% phantom-uncovered that disappears under
nightly. Stable still drives build/test/CI; this is a workflow-only nightly use.

Two failure modes read as coverage results but are not:

- **0% for every file is a tooling failure**, never a real number — it means `cargo-llvm-cov`
  is older than the nightly. Chasing it as a regression wastes a session. Cross-check with
  stable `cargo llvm-cov --lib --all-features`, which keeps working.
- **A `const fn` called only from `const` initializers reads 0%.** llvm-cov instruments
  runtime paths only; const evaluation is invisible to it. One trivial runtime test fixes it.

A percentage means nothing without its baseline. Before claiming a module regressed, stash the
branch and measure main — `src/proto/decoders.rs` read 85% on a PR branch against 41.7% on
main.

For a single module, the targeted `--no-report` + `jq` loop is seconds; `just cover` renders
everything and bounces you to a browser.

## Precedents

- #540 — `ProtocolFeature::new` looked covered (every `Features::*` constant invokes it) but
  reported 0%. The runtime test at `src/protocol_tests.rs:5` is the fix, and the reason that
  line exists.
- #554 — `contracts/types.rs` 61.4% → 99.6%, via [clock seams](clock-seams.md) plus serde
  round-trips and exercising both `&str` and `String` monomorphizations of `impl Into<T>`.
