---
id: plans-and-issues
title: plans/ is local scratch for implementation; open work lives in GitHub issues
cluster: workflow
status: active
triggers:
  - writing an implementation plan before coding
  - deferring a review finding, follow-up, or open question past the current PR
  - recording wire observations or investigation findings that outlive a PR
  - citing a plan file from code, docs, or a rule node
  - an issue is waiting on a gateway capture or account setup (`blocked`)
symbols: [plans/, .gitignore, docs/block-issues.md]
related: [pre-pr-checks]
precedents: ["#889", "#890"]
memory: [feedback_plans_in_todos]
---

`plans/` is git-ignored. Use it for the plan of the change you are about to implement, and
delete or let it go stale once the PR merges; the PR description and history carry what was
decided.

Anything that outlives the PR goes in a **GitHub issue**, not a plan file:

- deferred review or `/simplify` findings, rule-of-three deferrals, and follow-ups;
- open questions blocked on a decision;
- investigation findings and wire observations still needed by future work.

The exception is work blocked on something outside the code: a capture from a gateway in a
particular configuration, or account setup. It goes in [`docs/block-issues.md`](../../block-issues.md),
one section per item, each saying what unblocks it. When it unblocks, open an issue from the
section (or do the work) and delete the section.

Group related small items into one issue; give a bug or a decision its own. Cross-link issues
that must be sequenced. When a PR resolves one, close it from the PR (`Closes #N`).

Code comments, docs, and rule nodes cite the issue (`#N`), never a `plans/` path: the file
is not in the repo, so the reference is dead for every other reader.

## Why

Tracked follow-up files in `plans/` drifted: items shipped without being removed, sections
were renumbered under cross-references, and the files needed periodic pruning PRs (#890).
Source comments and rule nodes linked to them, so pruning a plan broke those links. Issues
close with the PR that fixes them, are searchable, and stay addressable after they close.

On 2026-10-02 the remaining open items moved to #891–#907, and the #789 design is linked from
that issue. On 2026-10-04 the two `blocked` issues (#904, #905) moved to `docs/block-issues.md`.
