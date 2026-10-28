---
id: feature-matrix
title: Three feature configurations must build and test, and `--features sync` is not one of them
cluster: parity
status: active
triggers:
  - touching code behind a cfg(feature) gate
  - adding a doctest or example to lib.rs
  - the Coverage job is red with no obvious code change
  - a type or impl exists on only one of the sync/async sides
symbols: [cfg(feature), no-default-features, just-test, default]
related: [dual-feature-types, no-parity-wrappers]
precedents: ["#658", "#671", "#724", "#744"]
memory: [feedback_ci_clippy_flags, feedback_fix_workspace_red_in_scope]
---

Three configurations must compile **and** pass tests when you touch feature-gated code:

```bash
cargo test                                            # default: async only
cargo test --no-default-features --features sync      # sync only
cargo test --all-features                             # sync + async + utoipa-6
```

**`--features sync` is not the sync-only build.** `default = ["async"]` and features are
additive, so it means sync **and** async. Only `--no-default-features` drops async.

`just test` runs all three. `ci.yml` has one matrix leg per configuration (`async`, `sync`,
`all-features`), each running build → clippy → test → examples → doc, with flags spelled out
rather than interpolated from a feature name so the trap cannot return.

## Why

Before #724, `just test` and `ci.yml` both used `--features sync` / `--features async`, so
sync-only was **never** built on a PR. Only `coverage.yml` caught it, and that runs on push to
`main`, after merge.

`lib.rs` is the usual casualty: an unguarded `#[tokio::main]` doctest there breaks sync-only.
Gate per-configuration doc examples with `cfg_attr`.

The legs fail in different directions: async-only catches an item silently depending on `sync`,
sync-only the reverse, `--all-features` same-name collisions (see
[dual-feature types](dual-feature-types.md)).

**Three green legs do not mean the sides agree.** Nothing compares a sync implementation against
its async twin. `historical_data` hand-rolled retry on each side and async retried the wrong case:
a routed `ConnectionReset` returned on first occurrence while a closed stream was re-sent five
times. Both sides had a test named `test_historical_data_connection_reset_after_retries` asserting
opposite outcomes. **Sibling tests with the same name and contradictory assertions are a drift
report, not coverage.** Read the pair side by side.

## Precedents

- #658 — unconditional `#[tokio::main]` doctest in `lib.rs`; sync-only doctests broke, all PR
  checks passed.
- #671 — the fix, five days later, while Coverage was red: `cfg_attr`-gated async and sync forms
  of each `lib.rs` example.
- #724 — three explicit legs in `just test` and `ci.yml`; swept `docs/build-and-test.md`, which
  had taught `--features sync` as "the sync build".
- #744 — moved `historical_data` onto the shared `retry_on_connection_reset`, removing the drifted
  pair. Found by reading, not by a gate.
