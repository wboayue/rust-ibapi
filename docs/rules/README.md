# Rule nodes

Project conventions, one directive per file. `CLAUDE.md` carries a trigger-phrased index;
the detail lives here so it loads only when the work actually calls for it.

## Why this shape

Each rule used to be two documents fused together — a directive and an evidence trail. The
directive is what you need at the moment of writing code; the evidence is what you need when
deciding whether the directive still applies. Splitting them lets the index stay short
enough to keep in context permanently while the reasoning stays available on demand.

## When a convention changes

The obligations — when a node is created, rewritten, or retired, and what has to be re-checked
alongside it — are in **[Maintaining the rule graph](../../CLAUDE.md#maintaining-the-rule-graph)**,
because they have to fire without opening this file. This page carries the mechanics they refer
to: the node format below, `triggers` phrasing, `status`, links, and the validator.

The reason those obligations are worth their space in permanent context: nothing here is
compile-checked, so a node that no longer matches `src/` is indistinguishable from one that
does until someone reads both. That is how five errors accumulated across rules 15–20 before
the first audit found them, and every cluster audited since has had the same class — names
decay quietly, and completeness claims ("fully applied", "zero remaining", "every `pub fn`")
decay faster than names.

## Node format

```markdown
---
id: proto-aware-accessors          # matches the filename stem
title: ResponseMessage accessors must be proto-aware
cluster: wire
status: active                     # active | historical
triggers:                          # situations, not topics — see below
  - adding a &self accessor on ResponseMessage
  - subscription routing silently returns no data
symbols: [ResponseMessage, peek_int, text_request_id_field]
related: [proto-only-decoding]     # ids of other nodes, not paths
precedents: ["#519", "#647"]       # PR numbers
memory: [feedback_request_id_index_registration]
---

The directive. Three to five lines, imperative, no history.

## Why
The bug class, and why the obvious approach fails.

## Precedents
- #647 — one line on what it established.
```

### `triggers` is the load-bearing field

It serves two purposes: it is the retrieval cue, and it is what the `CLAUDE.md` index line
is compressed from. Phrase entries as **situations you would recognise mid-task**, not as
topic labels. "Touching a domain decoder" fires; "decoder dispatch" does not.

### `status`

`historical` nodes are kept for archaeology and stay **out** of the `CLAUDE.md` index — a
concluded migration has no trigger, so loading it costs context and returns nothing. Keep
them when the reasoning would otherwise have to be reconstructed from PR archaeology, and
when a future change could reopen the arc.

### Links

- Node-to-node links use ordinary relative markdown links so they resolve for a human
  reading on GitHub. No `[[wikilinks]]`.
- `related` holds ids; the validator resolves them.
- `memory` names files in the maintainer's private memory directory. It is **not** validated
  — that directory lives outside the repo and is user-specific.

### ⚠️ `CLAUDE.md` must never `@`-import these files

Claude Code inlines `@path` imports recursively. An `@docs/rules/...` line would pull every
node into context on every session, which is exactly the cost this structure exists to
avoid. Use plain markdown links. `just rules-check` enforces this.

## Validation

```bash
just rules-check
```

Checks that every markdown link resolves, every `id` matches its filename, every `related`
id names a real node, that `CLAUDE.md` contains no `@`-imports, and that no node cites a
`file.rs:NNN` line number.

Link resolution covers `CLAUDE.md`, `docs/rules/**`, **and `plans/*.md`** — plan files cite
nodes by relative path, and those links rot as readily as the ones inside the graph.

**Cite names, not line numbers.** `src/protocol_tests.rs:5` is accurate until the next edit
to that file and nothing recomputes it — the same silent decay that made rule numbers
untrustworthy. Name the fn, test, or type and let the reader grep; the validator rejects
`.rs` / `.sh` / `.toml` / `.md` line suffixes in nodes.

## Clusters

| Cluster | Covers |
|---|---|
| `wire/` | Protobuf transport: decoders, accessors, wire-format typing |
| `style/` | How code is shaped: module layout, signatures, visibility, macros |
| `testing/` | Fixtures, coverage, what a test must exercise |
| `parity/` | The sync/async axis: feature configurations, dual-feature types, consumer idioms |
| `workflow/` | How a change gets made and shipped: local gates, toolchain, scope, restriction ordering |
| `docs/` | What ships alongside the code: rustdoc examples, changelog, migration guide, release notes |

## Retired rule numbers

`CLAUDE.md` carries no inline rules and no numbering. The numbering shifted at least once while
it was in use, so a "rule N" citation surviving in an old comment, plan, or memory has to be
resolved against the `CLAUDE.md` of **its own date** (`git show <commit>:CLAUDE.md`) — this
table is the mapping as of the final numbered revision, not a universal key.

| Rule | Node | Rule | Node |
|---|---|---|---|
| 1 | `parity/feature-matrix` | 15 | `wire/proto-only-decoding` |
| 2 | `style/domain-module-layout` | 16 | `wire/enum-typing` |
| 3 | `workflow/pre-pr-checks` | 17 | `wire/proto-aware-accessors` |
| 4 | `style/param-budget` | 18 | `docs/public-api-examples` |
| 5 | `parity/no-block-on` | 19 | `testing/fixture-builders` + `wire/fixture-migration` |
| 6 | `testing/coverage-floor` | 20 | `wire/floor-ratchet-splits` (historical) |
| 7 | `workflow/pinned-toolchain` | 21 | `testing/derive-from-constants` |
| 8 | `testing/sibling-test-files` | 22 | `testing/pin-compile-fail-codes` |
| 9 | `workflow/modernize-touched-modules` | 23 | `workflow/restrict-after-callers` |
| 10 | `testing/exercise-production-code` | 24 | `parity/subscription-consumer-idiom` |
| 11 | `workflow/integration-crate-builds` | 25 | `style/macros-last-resort` |
| 12 | `parity/dual-feature-types` | 26 | `testing/clock-seams` |
| 13 | `parity/no-parity-wrappers` | 27 | `docs/doc-parity-audit` |
| 14 | `style/narrow-reexports` | | |

Three unnumbered `CLAUDE.md` sections migrated alongside them — `docs/changelog-entry`,
`docs/release-notes`, `docs/user-docs-sync`. Rule 19 split deliberately: the text→proto fixture
sweep is concluded (`historical`), its builder-placement conventions are live. Two nodes came
from outside the numbered set: `wire/one-shot-narrowing` (#738/#749) and
`style/builder-enum-coverage` (established by #549, never written down until the sweep).

## Retrieval check

The regression check on trigger phrasing: re-run after a round of node edits, or whenever a
node's `triggers` are rewritten. Give each probe to a fresh subagent with no knowledge of the
graph. **Last run 2026-08-06: 3/3 pass** — every one opened its node as its first read rather
than answering from the `CLAUDE.md` index line.

1. *"I'm adding a `contract_id()` accessor to `ResponseMessage` — what do I need to know?"*
   → must surface the `raw_bytes`-first branch **and** the `text_request_id_field` registration.
2. *"Should `OrderState.completed_status` be a typed enum?"*
   → must surface verify-the-wire-first and reach **no** (the wire carries free-form text).
3. *"Where do response test fixtures go?"*
   → must name `src/testdata/builders/<domain>.rs`.

**Fail criterion:** answering from the index line alone, without opening the node, on 2 of 3
probes ⇒ the trigger phrasing is too weak. One session is noisy evidence; re-run if ambiguous.

**This file contaminates its own probes** — it lists both the questions and the expected
answers, and probe agents grep the repo. Hold the probes outside the tree on a re-run or the
result means nothing.
