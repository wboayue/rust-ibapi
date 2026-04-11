# Phase 1: Make `prost` a Required Dependency

## Context

The `proto` module is behind a feature gate (`proto = ["dep:prost"]`). Since v3.0 is protobuf-only, `prost` becomes a required dependency and the feature gate is removed.

## Changes

### 1. `Cargo.toml`

**Remove feature:**
```toml
# Delete this line from [features]:
proto = ["dep:prost"]
```

**Make prost required:**
```toml
# Change from:
prost = { version = "0.14", optional = true }
# To:
prost = "0.14"
```

### 2. `src/lib.rs` (line 129)

**Remove feature gate:**
```rust
// Change from:
#[cfg(feature = "proto")]
pub mod proto;

// To:
pub mod proto;
```

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Remove `proto` feature, make `prost` required |
| `src/lib.rs` | Remove `#[cfg(feature = "proto")]` on `pub mod proto` |

## Verification

```bash
cargo build
cargo build --features sync
cargo build --all-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
```
