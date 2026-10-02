# Migration Guide: 4.x to 5.0

This guide covers the breaking changes in `ibapi` 5.0. For 3.x → 4.x, see [`migration-4.0.md`](migration-4.0.md).

## Breaking changes

### 1. The `utoipa` feature requires utoipa 6

With the `utoipa` feature enabled, public types derive `utoipa::ToSchema`, so utoipa is part of the public API. 5.0 moves from utoipa 5 to utoipa 6. The two versions' `ToSchema` traits are distinct, so a crate still on utoipa 5 that names `ibapi` types in a `#[derive(OpenApi)]` or `#[schema(...)]` no longer compiles: our types implement utoipa 6's `ToSchema`, not the one it imports.

Move your own utoipa dependency, and any utoipa integration crates, to versions built on utoipa 6:

```toml
# Cargo.toml
utoipa = "6"
```

utoipa 6 needs Rust 1.88 or later. The schemas `ibapi` produces are unchanged. Users without the `utoipa` feature are not affected.

## Quick migration checklist

1. If you enable the `utoipa` feature, upgrade to utoipa 6 — see [§1](#1-the-utoipa-feature-requires-utoipa-6).
2. Re-run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and your test suite for each feature flag you support.

## Need help?

- Examples: `examples/async` and `examples/sync`
- Issues: <https://github.com/wboayue/rust-ibapi/issues>
