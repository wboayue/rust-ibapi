---
id: public-api-examples
title: Every public entry point carries a # Examples block
cluster: docs
status: active
triggers:
  - adding a pub fn, pub constructor, or public builder entry point
  - writing rustdoc for a new API
  - a reviewer calls an example redundant with the builder's
symbols: [Examples, no_run]
related: [doc-parity-audit, param-budget, user-docs-sync]
precedents: ["#657", "#659"]
memory: [feedback_doc_example_user_trace]
---

Every `pub fn`, `pub` constructor, and public builder entry point gets a `# Examples` heading
followed by a runnable example — `no_run` is the norm, `ignore` is acceptable — showing the
canonical happy-path call.

The heading is part of it. A bare fenced block still compiles under `cargo test --doc`, but
docs.rs renders no "Examples" section for it, and the file stops matching its 128 siblings.
Spell it `# Examples`, plural, even when there is one.

Exempt: struct field getters and trivial `is_*` predicates. An example on those is noise.

**Not exempt:** a builder entry point whose terminal action already has one. `client.foo(&c)`
and `.subscribe()` are different surfaces — the entry point teaches which builder you are in,
the terminal teaches what comes back.

## Why

The example is a contract in three directions at once. It teaches the idiom, it is compiled by
`cargo test --doc` so a signature change that breaks callers breaks the build, and it is the
first thing a user sees on docs.rs.

Write it from what the user *observes*, not from what compiles. `drop(subscription)` and a
`Ok(_) => {}` catch-all are the canary anti-patterns — both compile, neither shows anything.
An example that subscribes should print or match on what arrives.

Doc-examples are also a parity surface: match the sync counterpart of the *same method* rather
than the other examples in the same file. See [doc parity audit](doc-parity-audit.md).

## Coverage today

128 of 153 `impl Client` methods carry `# Examples`. The gap is inventoried in
[plans/code-consistency-followups.md](../../../plans/code-consistency-followups.md) — eight
genuine misses (`exercise_options`, `market_rule`, `family_codes`, `server_time_millis`,
`cancel_historical_ticks`, `cancel_contract_details`, and async `market_data`), plus the
trivial client accessors that are exempt. Take one when you are already in the file.

## Precedents

- #657 / #659 — the sweep that brought the async side up to the sync side's example coverage.
