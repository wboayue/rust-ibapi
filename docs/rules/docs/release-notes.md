---
id: release-notes
title: Release notes are changelog entries plus a code sample and an attribution
cluster: docs
status: historical
triggers:
  - drafting GitHub release notes for a tag
symbols: []
related: [changelog-entry]
precedents: ["#706", "#707"]
memory: []
---

**Moved.** Release procedure and the release-notes format now live in the `/release` skill:
[`.claude/skills/release/SKILL.md`](../../../.claude/skills/release/SKILL.md) and
[`references/release-notes.md`](../../../.claude/skills/release/references/release-notes.md).
Skills load on invocation, so the release process no longer needs an index line in
`AGENTS.md`. This node is kept for its precedents.

## Precedents

- v3.2.0 — both attribution forms in one release: a contribution credit on a
  `WhatToShow::AggTrades` PR, a report credit on a log-level fix.
- v3.3.0 (#706, #707) — the multi-PR heading form, for a feature spanning a read and a
  write API.
- v5.2.0 — the current layout: summary paragraph, grouped fixes, CHANGELOG anchor link,
  closing thanks for internal work.
