---
id: pin-compile-fail-codes
title: Pin compile_fail doc-tests to an error code
cluster: testing
status: active
triggers:
  - writing a doc-test that asserts something does not compile
  - guarding a typestate builder terminal
  - reviewing a bare compile_fail annotation
symbols: [compile_fail, FuturesBuilder, Missing]
related: [exercise-production-code]
precedents: ["#548", "#845"]
memory: []
---

Always pin a `compile_fail` doc-test to the specific error code. A bare `compile_fail` passes
for **any** compilation failure — a renamed import, a missing trait, a rustc diagnostic change.

```rust
/// ```compile_fail,E0599
/// use ibapi::contracts::Contract;
///
/// let es = Contract::futures("ES").build(); // no `build` without a month terminal
/// ```
```

Find the code by dropping the annotation, compiling the snippet, and reading the `error[ENNNN]`
line.

## Why

An unpinned block whose `use` line goes stale fails for that reason alone and keeps reporting
green; the invariant is unguarded from then on, invisibly.

Current sites (`grep -rn compile_fail --include=*.rs src`):

- `src/contracts/mod.rs` — `Contract::futures` returns `FuturesBuilder<Symbol, Missing>`, and
  `build()` exists only after a month terminal. `E0599` ("no method named `build`") is the
  whole assertion; unpinned, it would also pass if `Contract` were renamed.
- `src/orders/builder/condition_helpers.rs` — `price(OrderId::new(1), ..)` must fail with
  `E0277` (unsatisfied trait bound), not a missing import.

## Precedents

- #548 — `Contract` lockdown PR that established pinning. (The pre-migration `CLAUDE.md` cited
  it as pinning `E0639` for a `#[non_exhaustive]` struct; no such doc-test ever existed.)
- #845 — the `E0277` id-type guard on `price`.
