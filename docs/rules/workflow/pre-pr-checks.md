---
id: pre-pr-checks
title: Run the full local gate before opening a PR
cluster: workflow
status: active
triggers:
  - about to commit or open a pull request
  - finished a change and wondering what to run
  - CI failed on rustdoc, an example build, or one feature leg only
symbols: [cargo-fmt, cargo-clippy, RUSTDOCFLAGS, just-test, just-rules-check]
related: [feature-matrix, integration-crate-builds, coverage-floor, pinned-toolchain]
precedents: ["#724", "#725"]
memory: [feedback_ci_clippy_flags, feedback_rustdoc_link_check, feedback_self_review]
---

Before opening a PR, run every gate — formatter, then clippy, rustdoc, and examples in all three
feature configurations, plus the tests:

```bash
cargo fmt

cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features sync -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings

RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

just test

cargo build --examples
cargo build --examples --no-default-features --features sync
cargo build --examples --all-features
```

Add `just rules-check` when the change touches `AGENTS.md` or `docs/rules/`, and the
[integration crate builds](integration-crate-builds.md) when it touches a wire surface.

## Why

- **Rustdoc is the only check for intra-doc links.** `cargo test --doc` compiles `# Examples`
  bodies; it says nothing about whether a `[Client::stubbed]`-style link resolves.
- **`just test` does not build `examples/`**, and a signature change breaks examples as readily
  as callers.
- **The three configurations are not interchangeable**; `--features sync` is not the sync-only
  build. See [feature matrix](../parity/feature-matrix.md).

**Know which gates have no safety net.** `ci.yml` re-runs, on each of the three legs,
`cargo fmt -- --check`, the clippy, `cargo test`, `cargo build --examples`, and `cargo doc`
commands above; `basic-checks` runs the rules-graph script; `msrv` checks the library on 1.88
(CI-only — see [pinned toolchain](pinned-toolchain.md)). Nothing re-runs the
[integration crate builds](integration-crate-builds.md): skip those and the breakage ships.
Run the CI-covered gates locally anyway, on the pinned toolchain — one local loop beats a
push-wait-red-fix cycle.

## Precedents

- #724, #725 — the clippy list in `CLAUDE.md`, `just test`, and `ci.yml` all had the
  additive-features bug (the "sync" config was sync **plus** async); all now spell out one leg
  per configuration.
