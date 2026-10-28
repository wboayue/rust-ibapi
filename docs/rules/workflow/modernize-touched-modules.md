---
id: modernize-touched-modules
title: Bring the whole module up to convention when you touch it
cluster: workflow
status: active
triggers:
  - editing a module that still has inline tests or old idioms
  - a fix touches one function in a module that has drifted
  - deciding whether a cleanup belongs in this PR or its own
symbols: []
related: [sibling-test-files, restrict-after-callers]
precedents: ["#573", "#657"]
memory: [feedback_sed_inline_test_extraction, feedback_fix_workspace_red_in_scope]
---

When you modify a module for a feature or fix, bring the rest of that module up to current
convention in the same PR — extract inline `#[cfg(test)] mod tests` to a sibling `_tests.rs`,
fix small style drift, normalize patterns. Don't leave a module half-migrated. Likewise, fix
small pre-existing breakage your change surfaces rather than filing it.

Two boundaries:

- A **large mechanical sweep unrelated to the feature** gets its own PR. Test: would a reviewer
  ask "why is this here"? One module's cleanup alongside its fix reads as finishing the job;
  forty files of the same edit does not.
- **Adding a compile-time restriction runs the other way** — modernize callers first, in their
  own PR, then restrict. See [restrict after callers](restrict-after-callers.md).

## Why

Half-migrated modules are the expensive state: every later reader has to work out which half
is current, and the stale half gets copied. Finishing while the file is open costs minutes; a
separate cleanup PR costs a review cycle nobody schedules.

## Precedents

- #657 — the bulk inline-test extraction sweep; it left `mod tests;` declarations resolving to
  `<dir>/tests.rs`, the kind of leftover this rule converts one at a time. Current count in
  [sibling test files](../testing/sibling-test-files.md).
- #573 — asked for doc parity on one module, shipped docs plus a signature reshape, a method
  split, and a parameter rename, all drift in the touched module. Generalized as the
  [doc parity audit](../docs/doc-parity-audit.md).
