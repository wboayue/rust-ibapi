---
id: changelog-entry
title: A user-facing change adds its CHANGELOG entry in the same PR
cluster: docs
status: active
triggers:
  - opening a PR that changes public behavior or API
  - unsure whether a change is user-facing
  - cutting a release
symbols: [CHANGELOG.md, Unreleased]
related: [release-notes, user-docs-sync, restrict-after-callers]
precedents: ["#677", "#716"]
memory: []
---

Root `CHANGELOG.md` follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and
[SemVer](https://semver.org/spec/v2.0.0.html). Every PR with a user-facing change adds its
entry under `## [Unreleased]` **in the same PR** — a stale changelog blocks the same way a
stale README does.

- Group under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. Omit empty
  groups; keep the survivors in that order.
- One bullet per change, imperative and concise, ending with the PR number:
  `- Classify TWS codes 10089/10167 as informational so delayed-data subscriptions stay open (#677).`
- Breaking changes go under `Changed` or `Removed` **and** get a section in
  `docs/migration-3.0.md` — see [user docs sync](user-docs-sync.md).

Internal-only work needs no entry: refactors, tests, CI, doc-only edits, dependency bumps with
no behavior change. The test is "would a downstream user notice?" If no, skip it.

## On release

1. Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (ISO 8601).
2. Add a fresh empty `## [Unreleased]` above it. Version sections stay newest-first.
3. Update the link-reference definitions at the bottom — `[Unreleased]` →
   `compare/vX.Y.Z...HEAD`, `[X.Y.Z]` → its compare or tag URL.

## Why

The changelog is the append-as-you-go record; [release notes](release-notes.md) are the same
entries expanded with code samples at release time. Writing it at release time instead means
reconstructing intent from merge commits, which is when "silently truncated fractional sizes"
becomes "fix decimal parsing" and the reader learns nothing.

Keep the two consistent — a release note with no changelog bullet behind it is a sign one of
them was written from the diff rather than from the change.
