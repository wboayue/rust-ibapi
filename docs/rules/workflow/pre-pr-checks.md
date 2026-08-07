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
rustdoc in all three, and the tests:

```bash
cargo fmt

cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features sync -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings

RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sync
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

just test
```

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

The rest is CI parity rather than novelty: `ci.yml` runs `cargo fmt -- --check`,
`cargo clippy --all-targets <flags> -- -D warnings`, and `cargo test <flags>` on each of the
three legs, plus `just rules-check` in `basic-checks`. Running them locally costs one loop
instead of a push-wait-red-fix cycle, on the same toolchain CI uses — see
[pinned toolchain](pinned-toolchain.md).

## Precedents

- #724, #725 — the clippy trio in `CLAUDE.md` had the additive-features bug (its middle config
  was sync **plus** async), and `just test` / `ci.yml` had it too. All three now spell out one
  leg per configuration.
