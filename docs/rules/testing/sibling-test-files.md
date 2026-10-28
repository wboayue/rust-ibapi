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
precedents: ["#657", "#726", "#1042"]
memory: [feedback_sed_inline_test_extraction, project_main_flat_helpers_nested_pattern]
---

Never write an inline `#[cfg(test)] mod tests { ... }` block in an implementation file. Tests
go in a flat sibling file — `foo.rs` + `foo_tests.rs`, not `foo/mod.rs` + `foo/tests.rs`. The
test file opens with `use super::*;`.

Wire it in from the implementation file (or the parent `mod.rs`, for domain submodules):

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

## Why

The project minimizes `mod.rs` files, and inline tests make every implementation file longer
than the code it holds. No implementation file carries an inline test block, so a new one is
drift. (Grouping `mod` blocks *inside* a `*_tests.rs` file is fine, as is the test-support code
in `src/common/test_utils.rs`.)

Residue: four `tests.rs` files remain (`src/tests.rs`, `src/messages/tests.rs`,
`src/transport/tests.rs`, `src/orders/tests.rs`) beside 118 `*_tests.rs` (2026-10-27;
`find src -name tests.rs`, `find src -name '*_tests.rs' | wc -l`). Convert one when already
editing that module — see [modernize touched modules](../workflow/modernize-touched-modules.md) —
and do not treat them as precedent.

This does **not** mean flattening helper modules: `<domain>/<side>.rs` with
`<domain>/<side>/foo.rs` helpers underneath stays. Only test-file placement is governed here.

When sweeping ten or more inline blocks at once, `sed`-based extraction beats per-file
Read + Write + Edit.

## Precedents

- #657 — bulk extraction sweep; established the `sed` recipe.
- #726 — moved the last inline block (`from_str_tests` out of `src/messages.rs`) and corrected
  this node's "fully applied" claim: the `<dir>/tests.rs` residue had gone unnoticed.
- #1042 — flattened the directories holding only `tests.rs` (plus `mod.rs`) to `foo.rs` +
  `foo_tests.rs`; renamed `market_data/tests.rs` to `mod_tests.rs`.
