---
id: release-notes
title: Release notes are changelog entries plus a code sample and an attribution
cluster: docs
status: active
triggers:
  - drafting GitHub release notes for a tag
  - cutting a release
symbols: []
related: [changelog-entry, user-docs-sync]
precedents: ["#706", "#707"]
memory: [feedback_release_notes]
---

Format for GitHub release notes:

- Group under `## What's New` and `## Bug Fixes`, whichever apply.
- Each item is an `### H3` naming the change and its PR number(s) —
  `### Feature name (#123)`, or `(#707, #708)` when several PRs landed one feature.
- One sentence of summary under the heading.
- A fenced ```rust block showing typical usage.
- Order items by significance, most impactful first.

**Attribute every outside contribution.** Look up the PR author and the issue reporter with
`gh pr view` / `gh issue view` and close the item with `Thanks to @username for the
contribution.` or `Thanks to @username for the report.` This is the one thing the notes carry
that the changelog does not, and it is the reason to draft them by hand rather than generate
them from the bullets.

## Why

The changelog bullet answers "what changed"; the release note answers "what do I write now".
The code sample is what makes the difference — a reader scanning a release decides whether to
upgrade based on whether the new call looks like something they want to type.

Draft from the `## [Unreleased]` section of `CHANGELOG.md`, not from the merge log — see
[changelog entry](changelog-entry.md) for how those bullets are written. If a release note has
no changelog bullet behind it, the changelog was skipped in some PR.

## Precedents

- v3.2.0 — both attribution forms in one release: `Thanks to @bebop23 for the contribution.`
  on a `WhatToShow::AggTrades` PR, `Thanks to @thimo-seidel for the report.` on a log-level fix.
- v3.3.0 (#706, #707) — the multi-PR heading form, and the shape to copy for a feature that
  spans a read and a write API.
