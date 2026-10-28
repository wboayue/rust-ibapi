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

`rust-toolchain.toml` pins each branch to one Rust version — **1.95.0 on `main`, 1.93.0 on
`v2-stable`** — and `.github/workflows/ci.yml` pins `dtolnay/rust-toolchain@<same version>`.
Local and CI must run the same compiler, or clippy disagrees about what is a warning.

To upgrade: in one PR, bump `rust-toolchain.toml` and the `dtolnay/rust-toolchain@` pins of
**both** the `ci` and `basic-checks` jobs in `ci.yml`, fix the new lints, verify CI green.
**Leave the `msrv` job's `@1.88` pin alone** — it tracks `rust-version` in `Cargo.toml`, not the
toolchain. (`coverage.yml` comments also name the stable version; update them too.)

## Why

Bumping only the `ci` pin leaves `basic-checks` on the old compiler — the split the pin exists
to prevent. Bumping `msrv` with the toolchain means nothing checks the declared `rust-version`
any more; it runs `cargo +1.88 check -p ibapi --lib` on each feature leg.

Unpinned on purpose:

- `coverage.yml` uses `@nightly` and `cargo +nightly llvm-cov`, because rustdoc's
  `--persist-doctests` is nightly-only — see [coverage floor](../testing/coverage-floor.md).
  `+nightly` overrides `rust-toolchain.toml`. Failure mode: a `cargo-llvm-cov` older than the
  installed nightly reports 0% for every file rather than erroring.
- `security.yml` uses `@stable` — `cargo audit` / `cargo deny` track advisories, not MSRV.

`v2-stable` backports compile under 1.93.0, so code relying on a 1.94+ feature builds on `main`
and fails there. On `main` the library must still build under 1.88 (the `msrv` job).
