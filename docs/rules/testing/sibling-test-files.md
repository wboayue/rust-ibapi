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
related: [coverage-floor, exercise-production-code, modernize-touched-modules]
precedents: ["#657", "#726"]
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
file longer than the code it holds. No inline `#[cfg(test)] mod ... { }` block remains in
`src/`, so a new one is drift, not a judgment call.

The *placement* half is less finished than the count suggests: alongside the 85 flat
`*_tests.rs` files, 21 `mod tests;` declarations still resolve to a `<dir>/tests.rs`
(`find src -name "*_tests.rs" | wc -l` and `find src -name tests.rs | wc -l`, 2026-08-08).
Convert one when you are already editing that module — see
[modernize touched modules](../workflow/modernize-touched-modules.md) — and do not read the
surviving `<dir>/tests.rs` files as precedent.

This does **not** mean flattening helper modules. `<domain>/<side>.rs` as the main file with
`<domain>/<side>/foo.rs` helpers underneath is canonical Rust and stays; only the *test* file
placement is what this rule governs.

When sweeping ten or more existing inline blocks at once, `sed`-based extraction beats
Read + Write + Edit per file.

## Precedents

- #657 — the bulk extraction sweep that established the `sed` recipe.
- #726 — moved the last inline block (`from_str_tests` in `src/messages.rs`) out, and corrected
  this node's "fully applied" claim: the `<dir>/tests.rs` residue had gone unnoticed.
