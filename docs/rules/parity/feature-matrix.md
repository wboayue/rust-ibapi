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

`just test` runs all three, and `ci.yml` has one matrix leg per configuration (`async`, `sync`,
`all-features`), each running build → clippy → test → examples → doc. The legs spell their
flags out rather than interpolating a feature name, precisely so the additive-features trap
cannot come back.

## Why

The trap used to be load-bearing, because the project's own tooling fell into it. Until #724,
`just test` ran `--features sync` then `--features async` — async-only and sync-plus-async,
**never sync-only** — and `ci.yml`'s matrix was `feature: [sync, async]` with the same flag
shape and no `--all-features` leg. A sync-only break was invisible on the PR that introduced it
and on every PR after it; only `coverage.yml` caught it, and that runs `on: push` to `main`,
i.e. after merge.

`lib.rs` is the usual casualty: it is compiled in every configuration, so an unguarded
`#[tokio::main]` doctest there breaks sync-only. Gate per-configuration doc examples with
`cfg_attr` rather than assuming the async form.

Keep all three legs. They fail in different directions: async-only catches an item that
silently depends on `sync`, sync-only catches the reverse, and `--all-features` catches
same-name collisions that neither single-feature build can see — see
[dual-feature types](dual-feature-types.md).

## Precedents

- #658 — added a crate-level market-order doctest using `#[tokio::main]` unconditionally.
  Sync-only doctests stopped compiling; all PR checks passed.
- #671 — the fix, 11 commits and five days later. The Coverage job had been red the whole
  time. `cfg_attr`-gated async and sync forms of each `lib.rs` example.
- #724 — closed the gate gap that let #658 through: three explicit legs in `just test` and
  `ci.yml`, and swept `docs/build-and-test.md`, which had taught `--features sync` as "the
  sync build" throughout.
