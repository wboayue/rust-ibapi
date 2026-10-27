---
id: changelog-entry
title: A user-facing change adds its CHANGELOG entry in the same PR
cluster: docs
status: active
triggers:
  - opening a PR that changes public behavior or API
  - unsure whether a change is user-facing
symbols: [CHANGELOG.md, Unreleased]
related: [user-docs-sync, restrict-after-callers]
precedents: ["#677", "#716"]
memory: []
---

Root `CHANGELOG.md` follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and
[SemVer](https://semver.org/spec/v2.0.0.html). Every PR with a user-facing change adds its
entry under `## [Unreleased]` **in the same PR** — a stale changelog blocks the same way a
stale README does.

- Group under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. Omit empty
  groups; keep the survivors in that order.
- One bullet per change, imperative and concise, ending with the PR number. Verbatim from
  `CHANGELOG.md`:
  `- Delayed-data advisories (codes 10089/10167) no longer terminate a market-data subscription before its data arrives (#677).`
  Note what it leads with — the effect a user would notice, not the mechanism that produced it.
- Breaking changes go under `Changed` or `Removed` **and** get a section in
  `docs/migration-5.0.md` — see [user docs sync](user-docs-sync.md).

Internal-only work needs no entry: refactors, tests, CI, doc-only edits, dependency bumps with
no behavior change. The test is "would a downstream user notice?" If no, skip it.

## On release

Dating the section and updating the compare links is part of the `/release` skill
([`.claude/skills/release`](../../../.claude/skills/release/SKILL.md)).

## Why

The changelog is the append-as-you-go record; [release notes](../../../.claude/skills/release/references/release-notes.md) are the same
entries expanded with code samples at release time. Writing it at release time instead means
reconstructing intent from merge commits, which is when "silently truncated fractional sizes"
becomes "fix decimal parsing" and the reader learns nothing.

Keep the two consistent — a release note with no changelog bullet behind it is a sign one of
them was written from the diff rather than from the change.
