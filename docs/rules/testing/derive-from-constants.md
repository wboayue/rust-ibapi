---
id: derive-from-constants
title: Derive test expectations from the constant under test
cluster: testing
status: active
triggers:
  - asserting against a version-gated API
  - writing a boundary test around a minimum server version
  - a test hardcodes a wire number or a feature name
symbols: [Features, ProtocolFeature, server_versions, IncomingMessages, check_version]
related: [coverage-floor, exercise-production-code]
precedents: ["#540"]
memory: []
---

When a test asserts against a version-gated API — `Features::*`, `server_versions::*`,
`IncomingMessages::*` — bind the constant once and derive every assertion from it. Never
hardcode the wire value.

```rust
let feature = Features::CONFIG;
assert_eq!(*required, feature.min_version);   // not `219`
assert_eq!(name, feature.name);               // not `"configuration"`
```

Boundaries are `feature.min_version - 1` and `feature.min_version`. For `Display` assertions,
build the expected string with `format!()` over the same fields — it still checks ordering and
punctuation.

## Why

A hardcoded number decays silently: when a `MIN_SERVER_VER_*` moves, the constant moves and
the test still passes while no longer asserting what its name claims. `ProtocolFeature` has
exactly two fields, `name` and `min_version`, so everything a version test needs is reachable
from the bound constant. `src/protocol_tests.rs` is the worked example, including the
`format!()`-built `Display` assertion.

## Precedents

- #540 — /simplify caught three hardcoded sites; same PR found
  [`ProtocolFeature::new` reading 0% coverage](coverage-floor.md).
