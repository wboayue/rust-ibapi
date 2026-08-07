---
id: pre-pr-checks
title: Run the full local gate before opening a PR
cluster: workflow
status: active
triggers:
  - about to commit or open a pull request
  - finished a change and wondering what to run
  - CI is green but docs.rs links are broken
symbols: [cargo-fmt, cargo-clippy, RUSTDOCFLAGS, just-test, just-rules-check]
related: [feature-matrix, integration-crate-builds, coverage-floor, pinned-toolchain]
precedents: ["#724", "#725"]
memory: [feedback_ci_clippy_flags, feedback_rustdoc_link_check, feedback_self_review]
---

Before opening a PR, run every gate — formatter, clippy in all three feature configurations,
rustdoc in all three, the tests, and the examples:

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
```

`just test` does not build `examples/`, and a signature change breaks examples as readily as it
breaks callers — that is a whole caller surface the test suite never touches.

Add `just rules-check` when the change touches `CLAUDE.md`, `docs/rules/`, or `plans/`, and the
[integration crate builds](integration-crate-builds.md) when it touches a wire surface.

## Why

**The rustdoc trio is a local-only gate.** `ci.yml` runs `cargo doc --no-deps` per leg with no
`RUSTDOCFLAGS`, so a broken intra-doc link warns and passes. `cargo test --doc` does not cover
it either — that compiles the `# Examples` bodies, and says nothing about whether a
`[Client::stubbed]`-style intra-doc reference resolves. The first place a broken link becomes
visible is docs.rs.

The three feature configurations are not interchangeable and `--features sync` is not the
sync-only build; see [feature matrix](../parity/feature-matrix.md) for why, and for the
incident where a sync-only break survived 11 merges.

**Know which half of the list has no safety net.** `ci.yml` re-runs `cargo fmt -- --check`,
`cargo clippy --all-targets <flags> -- -D warnings`, `cargo test <flags>`, and
`cargo build --examples` on each of the three legs, and `basic-checks` runs the rules-graph
script — skip those locally and CI catches you. Nothing re-runs the rustdoc trio or the
[integration crate builds](integration-crate-builds.md); skip those and the breakage ships.

Run the CI-covered half locally anyway, on the same toolchain CI uses — see
[pinned toolchain](pinned-toolchain.md) — because one local loop beats a push-wait-red-fix
cycle.

## Precedents

- #724, #725 — the clippy trio in `CLAUDE.md` had the additive-features bug (its middle config
  was sync **plus** async), and `just test` / `ci.yml` had it too. All three now spell out one
  leg per configuration.
