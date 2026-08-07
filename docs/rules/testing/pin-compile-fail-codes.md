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
precedents: ["#548"]
memory: []
---

A bare `compile_fail` passes for **any** compilation failure — a renamed import, a missing
trait, a future rustc diagnostic change. Always pin the specific code so a regression in the
*guarded* behavior surfaces instead of the test passing for the wrong reason.

```rust
/// ```compile_fail,E0599
/// use ibapi::contracts::Contract;
///
/// let es = Contract::futures("ES").build(); // no `build` without a month terminal
/// ```
```

Find the code by dropping the annotation, running the snippet through `rustc`, and reading the
`error[ENNNN]` line.

## Why

The failure mode is a test that no longer tests anything while still reporting green. A
`compile_fail` block whose `use` line goes stale fails to compile for that reason alone and
keeps passing — the actual invariant is unguarded from then on, invisibly.

The repo's one instance is the typestate guard in `src/contracts/mod.rs`: `Contract::futures`
returns `FuturesBuilder<Symbol, Missing>`, and `build()` exists only once a month terminal has
been applied. `E0599` is "no method named `build` found" — that specific code is the whole
assertion. Without it the test would also pass if `Contract` were renamed.

## Precedents

- #548 — the `Contract` lockdown PR that established pinning.

> **Correction.** The pre-migration `CLAUDE.md` rule cited this precedent as pinning
> `compile_fail,E0639` for "cannot construct a `#[non_exhaustive]` struct externally". No such
> doc-test exists; the only `compile_fail` in the repo is the `E0599` typestate guard above.
> The directive was right, the evidence was not.
