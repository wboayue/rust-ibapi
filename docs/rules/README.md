# Rule nodes

Project conventions, one directive per file. `CLAUDE.md` carries a trigger-phrased index;
the detail lives here so it loads only when the work actually calls for it.

## Why this shape

Each rule used to be two documents fused together — a directive and an evidence trail. The
directive is what you need at the moment of writing code; the evidence is what you need when
deciding whether the directive still applies. Splitting them lets the index stay short
enough to keep in context permanently while the reasoning stays available on demand.

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
| `testing/` | Fixtures, coverage, what a test must exercise |
| `parity/` | The sync/async axis: feature configurations, dual-feature types, consumer idioms |

Remaining clusters (`workflow/`, `style/`, `docs/`) are still inline in `CLAUDE.md`. See
[plans/claude-md-knowledge-graph.md](../../plans/claude-md-knowledge-graph.md) for the
migration order.
