---
id: release-notes
title: Release notes are changelog entries plus a code sample and an attribution
cluster: docs
status: historical
triggers:
  - drafting GitHub release notes for a tag
symbols: []
related: [changelog-entry]
precedents: ["#707", "#708"]
memory: []
---

**Moved.** Release procedure and the release-notes format live in the `/release` skill:
[`.claude/skills/release/SKILL.md`](../../../.claude/skills/release/SKILL.md) and
[`references/release-notes.md`](../../../.claude/skills/release/references/release-notes.md).
Skills load on invocation, so this needs no `AGENTS.md` index line. Kept for its precedents.

## Precedents

- v3.2.0 — both attribution forms in one release: a contribution credit on the
  `WhatToShow::AggTrades` PR (#693), a report credit on a log-level fix (#695).
- v3.3.0 (#707, #708) — the multi-PR heading form, for a feature spanning a read and a write
  API (`Client::config()` / `update_config()`).
- v5.2.0 — the current layout: summary paragraph, grouped fixes, CHANGELOG anchor link,
  closing thanks for internal work.
