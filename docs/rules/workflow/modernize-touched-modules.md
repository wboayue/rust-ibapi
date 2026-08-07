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

When you modify a module for a feature or a fix, bring the rest of that module up to current
convention in the same PR — extract inline `#[cfg(test)] mod tests` to a sibling `_tests.rs`,
fix small style drift, normalize patterns. Be aggressive: do not leave a module half-migrated.

Two boundaries:

- A **large mechanical sweep unrelated to the feature** still gets its own PR. The test is
  whether a reviewer reading your diff would ask "why is this here" — one module's cleanup
  alongside its fix reads as finishing the job; forty files of the same edit does not.
- **Adding a compile-time restriction is the opposite order** — modernize callers first, in
  their own PR, then restrict. See [restrict after callers](restrict-after-callers.md).

## Why

Half-migrated modules are the expensive state. Every later reader has to work out which half is
current, and the stale half keeps getting copied — a module with inline tests teaches inline
tests to the next person who opens it. The marginal cost of finishing while the file is already
open is minutes; the cost of a separate cleanup PR is a review cycle nobody schedules.

The same instinct covers pre-existing breakage a PR surfaces: if the workspace is red in a
small way next to your change, fix it here rather than filing it.

## Precedents

- #657 — the bulk inline-test extraction sweep, and a good picture of what a sweep does and
  does not finish. It left 87 `#[path]`-wired sibling files, but about two dozen `mod tests;`
  declarations still resolve to a `<dir>/tests.rs` rather than a `_tests.rs` sibling. Those are
  the leftovers this rule is for: convert the one in front of you, not all of them. See
  [sibling test files](../testing/sibling-test-files.md).
- #573 — asked for doc parity on one module, shipped docs plus a signature reshape, a method
  split, and a parameter rename, because they were all drift in the module being touched. The
  doc-parity rule that generalises it is still inline in `CLAUDE.md` (rule 27).
