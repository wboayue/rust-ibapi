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
`ibapi-test` helper — are workspace members but not in `default-members` (`["."]`). Plain
`cargo build` / `cargo test` / `cargo clippy --all-targets` skip them, and so does every CI job.

When a change touches `Subscription`, the proto encoders or decoders, a public API shape, or
anything else wire-adjacent, also run:

```bash
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
cargo clippy -p ibapi-integration-sync  --tests -- -D warnings
cargo clippy -p ibapi-integration-async --tests -- -D warnings
```

Compilation is the check; no gateway needed. (`just integration` runs them against a live one.)

## Why

These crates are the largest body of real calling code in the repo and no automated gate
builds them. A signature change that compiles against `src/` can break every integration test
unnoticed until someone next runs them against a gateway. Deliberately caller-breaking changes
(`#[must_use]`, a removed field, narrowed visibility) are most exposed: their diff looks
complete because `cargo clippy --all-targets` is green.

## Precedents

- #593 — `#[must_use]` on 31 builders and 8 subscription types; eight of the nine callsites needing
  `let _ = ...` were integration-test cleanup paths, reachable only via `-p ibapi-integration-*`.
