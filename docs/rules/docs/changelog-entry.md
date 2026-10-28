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
precedents: ["#677", "#681", "#716"]
memory: []
---

Root `CHANGELOG.md` follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and
[SemVer](https://semver.org/spec/v2.0.0.html). Every PR with a user-facing change adds its
entry under `## [Unreleased]` **in the same PR**.

- Group under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`, in that order;
  omit empty groups.
- One bullet per change, ending with the PR number, leading with the effect a user would notice
  rather than the mechanism. From `CHANGELOG.md`:
  `- Delayed-data advisories (codes 10089/10167) no longer terminate a market-data subscription before its data arrives (#677).`
- Breaking changes go under `Changed` or `Removed` **and** get a migration-guide section — see
  [user docs sync](user-docs-sync.md) for which guide.

Internal-only work needs no entry: refactors, tests, CI, doc-only edits, dependency bumps with
no behavior change. Test: would a downstream user notice?

Dating the section and the compare links happen at release, in the `/release` skill
([`.claude/skills/release`](../../../.claude/skills/release/SKILL.md)).

## Why

[Release notes](../../../.claude/skills/release/references/release-notes.md) are drafted from
the dated changelog section. Writing entries at release time means reconstructing intent from
merge commits — "silently truncated fractional sizes" (#716) becomes "fix decimal parsing" and
the reader learns nothing. A release note with no changelog bullet behind it means a PR skipped
its entry.

## Precedents

- #681 — adopted Keep a Changelog 1.1.0.
- #677 — the model bullet quoted above: effect first, codes for searchability.
- #716 — the fractional-size fix, whose entries say what users saw (`0.5` decoded as `0`).
