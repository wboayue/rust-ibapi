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
`IncomingMessages::*` — bind the constant once and derive every assertion from its fields.
Never hardcode the wire value.

```rust
let feature = Features::TICK_BY_TICK;
assert_eq!(*required, feature.min_version);   // not `137`
assert_eq!(name, feature.name);               // not `"tick-by-tick data"`
```

Boundary cases then document themselves: `feature.min_version - 1` and `feature.min_version`.
For `Display` assertions, build the expected string with `format!()` against the same fields —
that still validates ordering and literal punctuation while parameterizing the values.

## Why

A hardcoded number decays silently. When IBKR bumps a `MIN_SERVER_VER_*`, the constant moves
and the test keeps passing — it is still syntactically valid, it just no longer asserts what
its name claims. There is no failure to notice, which is what makes this worse than a broken
test.

`ProtocolFeature` carries exactly two fields, `name` and `min_version`, so every assertion a
version test needs is reachable from the bound constant. `src/protocol_tests.rs` is the
worked example, including the `format!()`-built `Display` assertion.

## Precedents

- #540 — /simplify caught three hardcoded sites; the same PR is where
  [`ProtocolFeature::new` turned out to read 0% coverage](coverage-floor.md).
