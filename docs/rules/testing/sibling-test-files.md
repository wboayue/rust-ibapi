---
id: sibling-test-files
title: Tests live in flat sibling _tests.rs files, never inline
cluster: testing
status: active
triggers:
  - writing a #[cfg(test)] mod tests block
  - adding tests to a module that has none
  - sweeping inline test modules out of a file
symbols: [cfg(test), path-attribute]
related: [coverage-floor, exercise-production-code]
precedents: ["#657"]
memory: [feedback_sed_inline_test_extraction, project_main_flat_helpers_nested_pattern]
---

Never write an inline `#[cfg(test)] mod tests { ... }` block. Tests go in a flat sibling file
— `foo.rs` + `foo_tests.rs`, not `foo/mod.rs` + `foo/tests.rs`. The test file opens with
`use super::*;`.

Wire it in from the implementation file (or the parent `mod.rs`, for domain submodules):

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

## Why

The project minimizes `mod.rs` files, and an inline test module makes every implementation
file longer than the code it holds. The convention is fully applied: 87 `#[path = "*_tests.rs"]`
wirings, zero remaining inline `mod tests {` blocks. A new inline block is drift, not a
judgment call.

This does **not** mean flattening helper modules. `<domain>/<side>.rs` as the main file with
`<domain>/<side>/foo.rs` helpers underneath is canonical Rust and stays; only the *test* file
placement is what this rule governs.

When sweeping ten or more existing inline blocks at once, `sed`-based extraction beats
Read + Write + Edit per file.

## Precedents

- #657 — the bulk extraction sweep that established the `sed` recipe and closed out the last
  inline modules.
