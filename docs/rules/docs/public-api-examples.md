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
precedents: ["#657", "#659", "#751"]
memory: [feedback_doc_example_user_trace]
---

Every `pub fn`, `pub` constructor, and public builder entry point gets a `# Examples` heading
(plural, even for one) followed by a runnable example — `no_run` is the norm, `ignore`
acceptable — showing the canonical happy-path call. A bare fenced block still compiles under
`cargo test --doc`, but docs.rs renders no "Examples" section for it.

Exempt: field getters and trivial `is_*` predicates.

**Not exempt:** a builder entry point whose terminal already has one. `client.foo(&c)` and
`.subscribe()` are different surfaces — the entry point teaches which builder you are in, the
terminal what comes back.

## Why

The example teaches the idiom, is compiled by `cargo test --doc` (so a breaking signature change
breaks the build), and is the first thing a user sees on docs.rs.

Write it from what the user *observes*. `drop(subscription)` and an `Ok(_) => {}` catch-all are
the canary anti-patterns — both compile, neither shows anything. An example that subscribes
should print or match on what arrives. Pair it with the same method's sync/async counterpart;
see [doc parity audit](doc-parity-audit.md).

## Coverage today

Since #751 every `impl Client` method carries one except the exempt accessors (`client_id`,
`next_order_id`, `connection_time`, `time_zone`); so does every `orders::order_builder` free
function. A new method ships with its example. The singular `# Example` heading should not
reappear: `grep -rnE '^\s*//[/!] # Example\s*$' src | grep -v _tests.rs` prints nothing.

## Precedents

- #657 / #659 — brought the async side up to the sync side's example coverage.
- #751 — closed the backfill on the last eight `Client` methods.
