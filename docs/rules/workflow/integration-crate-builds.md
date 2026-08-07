---
id: integration-crate-builds
title: Build the integration crates when a change touches a wire surface
cluster: workflow
status: active
triggers:
  - changing Subscription, a proto encoder or decoder, or a public API shape
  - adding a compile-time restriction to a public type
  - wondering why cargo clippy --all-targets missed a broken call
symbols: [ibapi-integration-sync, ibapi-integration-async, ibapi-test, default-members]
related: [pre-pr-checks, restrict-after-callers, proto-only-decoding]
precedents: ["#593"]
memory: [feedback_minimal_test_helper_deps, feedback_integration_test_tws_encoding]
---

The `integration/` crates — `ibapi-integration-sync`, `ibapi-integration-async`, and the
`ibapi-test` helper — are workspace members but **not** in `default-members`, which is `["."]`.
Plain `cargo build` / `cargo test` / `cargo clippy --all-targets` skip them entirely, and so
does CI.

When a change touches `Subscription`, the proto encoders or decoders, a public API shape, or
anything else wire-adjacent, also run:

```bash
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
cargo clippy -p ibapi-integration-sync  --tests -- -D warnings
cargo clippy -p ibapi-integration-async --tests -- -D warnings
```

Compilation is the contract here — no live gateway is needed for this check.

## Why

These crates are the largest body of real calling code in the repo, and they are invisible to
every automated gate. A signature change that compiles against `src/` and its unit tests can
still break every integration test, and nothing will say so until someone runs them against a
gateway — which happens far less often than merging.

The exposure is worst for changes that are *deliberately* caller-breaking: a `#[must_use]`, a
removed field, a narrowed visibility. Those are exactly the PRs whose diff looks complete
because `cargo clippy --all-targets` is green.

## Precedents

- #593 — `#[must_use]` on 31 builders and 8 subscription types. Nine callsites had to gain an
  explicit `let _ = ...`; **eight of the nine were integration-test cleanup paths**, reachable
  only through `-p ibapi-integration-*`.
