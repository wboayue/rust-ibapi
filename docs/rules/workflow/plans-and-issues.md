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
precedents: ["#889", "#890", "#908"]
memory: [feedback_plans_in_todos]
---

`plans/` is git-ignored. Use it for the plan of the change you are about to implement; once the
PR merges, the PR description and history carry what was decided.

Anything that outlives the PR goes in a **GitHub issue**:

- deferred review or `/simplify` findings, rule-of-three deferrals, follow-ups;
- open questions blocked on a decision;
- investigation findings and wire observations future work still needs.

Group related small items into one issue; give a bug or a decision its own. Cross-link issues
that must be sequenced; close from the resolving PR (`Closes #N`).

Exception: work blocked on something outside the code (a gateway capture in a particular
configuration, account setup) goes in [`docs/block-issues.md`](../../block-issues.md), one
section per item saying what unblocks it. When it unblocks, open an issue (or do the work) and
delete the section.

Code comments, docs, and rule nodes cite the issue (`#N`), never a `plans/` path — the file is
not in the repo, so the reference is dead for every other reader.

## Why

Tracked plan files drifted: shipped items stayed listed, sections were renumbered under
cross-references, and pruning PRs broke the source comments and nodes that linked to them.
Issues close with the PR that fixes them, are searchable, and stay addressable after closing.

## Precedents

- #889 — a fix shipping its own tracked plan file (`plans/execution-alias-pruning.md`).
- #890 — a periodic pruning PR removing completed plan items.
- #908 — untracked `plans/`; open items moved to issues #891–#907. Blocked issues #904/#905
  later moved to `docs/block-issues.md`.
