---
id: feature-matrix
title: Three feature configurations must build and test, and CI covers only two
cluster: parity
status: active
triggers:
  - touching code behind a cfg(feature) gate
  - adding a doctest or example to lib.rs
  - the Coverage job is red with no obvious code change
  - a type or impl exists on only one of the sync/async sides
symbols: [cfg(feature), no-default-features, just-test, default]
related: [dual-feature-types, no-parity-wrappers]
precedents: ["#658", "#671"]
memory: [feedback_ci_clippy_flags, feedback_fix_workspace_red_in_scope]
---

Three configurations must compile **and** pass tests when you touch feature-gated code:

```bash
cargo test                                            # default: async only
cargo test --no-default-features --features sync      # sync only
cargo test --all-features                             # sync + async + utoipa
```

**`--features sync` does not give you the sync-only build.** `default = ["async"]`, and
feature flags are additive, so `--features sync` means *sync **and** async*. Only
`--no-default-features` drops the async client.

## Why

That trap is load-bearing, because the project's own tooling falls into it. `just test` runs
`cargo test --features sync` then `--features async`, so it covers async-only and
sync-plus-async — **never sync-only**. `ci.yml`'s matrix is `feature: [sync, async]` with the
same flag shape, so it has the same blind spot, and it has no `--all-features` leg at all.

What actually exercises sync-only:

| Gate | Runs sync-only? |
|---|---|
| `just test` | No — `--features sync` keeps async on |
| `ci.yml` (per PR) | No — same flag shape |
| `RUSTDOCFLAGS=… cargo doc … --no-default-features --features sync` | Yes, compile only |
| `coverage.yml` | Yes — but `on: push` to `main`, i.e. after merge |

So a sync-only break is invisible on the PR that introduces it and on every PR after it, until
someone reads the Coverage job. Run the sync-only line yourself; nothing upstream will.

`lib.rs` is the usual casualty: it is compiled in every configuration, so an unguarded
`#[tokio::main]` doctest there breaks sync-only while all three CI legs stay green. Gate
per-configuration doc examples with `cfg_attr` rather than assuming the async form.

## Precedents

- #658 — added a crate-level market-order doctest using `#[tokio::main]` unconditionally.
  Sync-only doctests stopped compiling; all PR checks passed.
- #671 — the fix, 11 commits and five days later. The Coverage job had been red the whole
  time. `cfg_attr`-gated async and sync forms of each `lib.rs` example.
