---
id: pinned-toolchain
title: The Rust toolchain is pinned; local and CI must agree
cluster: workflow
status: active
triggers:
  - a clippy lint fires locally that CI does not report, or vice versa
  - upgrading the Rust version
  - adding a GitHub Actions workflow that installs Rust
symbols: [rust-toolchain.toml, dtolnay/rust-toolchain, cargo-llvm-cov]
related: [pre-pr-checks, coverage-floor]
precedents: []
memory: [feedback_ci_clippy_flags, reference_llvm_cov_toolchain_pairing]
---

`rust-toolchain.toml` pins the branch to one Rust version — **1.95.0 on `main`, 1.93.0 on
`v2-stable`** — and `.github/workflows/ci.yml` pins `dtolnay/rust-toolchain@<same version>`.
Local and CI must run the same compiler, or clippy disagrees about what is a warning.

To upgrade: bump `rust-toolchain.toml` and **every** `dtolnay/rust-toolchain@` pin in
`ci.yml` in the same PR, fix the new lints, verify CI green.

## Why

`ci.yml` pins the version in **two** jobs — the feature matrix and `basic-checks`. Bumping
only the first leaves the second on the old compiler, which is exactly the split the pin
exists to prevent.

Not every workflow is pinned, deliberately:

- `coverage.yml` installs `@nightly` and runs `cargo +nightly llvm-cov`, because
  `--persist-doctests` is nightly-only — see [coverage floor](../testing/coverage-floor.md).
  `+nightly` overrides `rust-toolchain.toml`, so bumping the stable pin does not affect it.
  The failure mode is the *other* pairing: `cargo-llvm-cov` older than the installed nightly
  reports 0% for every file rather than erroring.
- `security.yml` uses `@stable` on purpose — `cargo audit` tracks advisories, not this crate's
  MSRV.

Because `rust-toolchain.toml` is per-branch, a `v2-stable` backport compiles under 1.93.0.
Code that relies on a 1.94+ feature builds on `main` and fails there.
