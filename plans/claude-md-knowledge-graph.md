# CLAUDE.md as a knowledge graph

Migration roadmap. `CLAUDE.md` becomes a trigger-phrased index; each directive becomes one
node under `docs/rules/` carrying its own evidence and typed edges.

**Status:** `wire/` cluster migrated (rules 15–20), plus one seeded `testing/` node. Five
clusters remain inline.

## Why

The 27-rule block was 3,733 words (~5.5k tokens) loaded into context every session, of which
a typical turn needs three rules. Three problems compounded:

1. **Each rule fused two documents** — a directive and an evidence trail. Rule 25 was 413
   words; the directive is about 30 and the rest a case log. Same shape in rules 27 (355w),
   19 (297w), 17 (294w), 23 (278w). The split is on the *directive vs. evidence* axis, not
   just by topic.
2. **Concluded history stayed loaded.** Rules 19 and 20 both self-described as "arc concluded
   at floor 213" — about 420 words with no live trigger. They are now `status: historical`
   and out of the index.
3. **The rules rotted silently.** Nothing compile-checks `CLAUDE.md`. An audit of rules 15–20
   against `src/` found five errors; a parallel survey found five contradictory sites in
   `docs/`, fixed in the preceding PR.

## What shipped in the wire/ pass

| Node | From | Status |
|---|---|---|
| `wire/proto-only-decoding.md` | rule 15 | active |
| `wire/proto-aware-accessors.md` | rule 17 | active |
| `wire/enum-typing.md` | rule 16 | active |
| `wire/fixture-migration.md` | rule 19 (sweep half) | historical |
| `wire/floor-ratchet-splits.md` | rule 20 | historical |
| `testing/fixture-builders.md` | rule 19 (builder half) | active |

Rule 19 split across clusters deliberately: the text→proto sweep is concluded, but its
builder-placement and field-minimal conventions are live and belong to `testing/`.

### Rot corrected while migrating

| Was | Is |
|---|---|
| `peek_string` cited as an accessor and as PR #519's bug root | **Does not exist.** `peek_int` is the only `peek_*` primitive |
| `parse_optional` described with a `label` param | Takes no label; only `parse_required` does |
| `Error::UnexpectedResponse` implied to carry a `ResponseMessage` | Carries a `String` since `ResponseMessage` became `pub(crate)` (#581) |
| "floor 213" as a bare number | `server_versions::PROTOBUF_REST_MESSAGES_3`. `PROTOBUF` is a *different*, lower constant (201) |
| `routes_by_request_id` described as a table | A function wrapping `text_request_id_field`, which is the source of truth |

## Retrieval check — run this before migrating further

**Not yet run.** It must happen in a session that did not design the graph, and it is the
gate on continuing. Three probes, each answerable only from a node:

1. *"I'm adding a `contract_id()` accessor to `ResponseMessage` — what do I need to know?"*
   → must surface the `raw_bytes`-first branch **and** the `text_request_id_field` registration.
2. *"Should `OrderState.completed_status` be a typed enum?"*
   → must surface verify-the-wire-first and reach **no** (the wire carries free-form text).
3. *"Where do response test fixtures go?"*
   → must name `src/testdata/builders/<domain>.rs`.

**Fail criterion:** answering from the index line alone, without opening the node, on 2 of 3
probes ⇒ the trigger phrasing is too weak. Revise before migrating another cluster. One
session is noisy evidence; re-run if the result is ambiguous.

## Remaining clusters

Migrate in this order — highest density and clearest boundaries first:

| Order | Cluster | Rules | Notes |
|---|---|---|---|
| 1 | `testing/` | 6, 8, 10, 21, 22, 26 | **Extend** the seeded directory; don't recreate it |
| 2 | `parity/` | 1, 5, 12, 13, 24 | sync/async axis |
| 3 | `workflow/` | 3, 7, 9, 11, 23 | |
| 4 | `style/` | 2, 4, 14, 25 | |
| 5 | `docs/` | 18, 27 + Changelog / Release Notes / Maintaining Documentation | |

**Do not renumber surviving rules.** The gap at 15–20 is deliberate. Rule numbers are already
unreliable across stores — one memory cites "rules 20/22" for the family now at 15/17, and
another references a *different* rule 19. `plans/code-consistency-followups.md` is organised
by rule number throughout. Numbering is retired at the end of migration, not incrementally.

## Follow-ups

- **Replace the two inline silent-failure warnings with a real gate.** A test asserting that
  every `IncomingMessages` variant reachable by a public API has a `text_request_id_field`
  entry would retire that clause from prose entirely. Prose is the weakest possible
  enforcement for a failure mode that passes CI.
- **Audit the remaining rules for rot** the way 15–20 were audited, before migrating each
  cluster. Assume they have it.
- **A stale rule-number citation exists in source:** `src/news/common/decoders.rs` cites
  "(rule 20)" for what is now the proto-only-decoding rule. Sweep for others when numbering
  is retired.
- **Reconcile the maintainer's memory store.** Two `[[wikilink]]` syntaxes coexist for the
  same targets (`[[project-protobuf-only]]` vs `[[project_protobuf_only]]`); the whole
  fixture/builder group sits outside the wikilink graph using backticked filenames; and one
  dangling `[[dual-feature-public-types]]` points at CLAUDE.md rule 12 — it resolves for free
  once rule 12 becomes `parity/dual-feature-types.md`.
- **Consider promoting clusters to subagents.** The node bodies would become the agent
  prompts. Stronger enforcement (the agent always has its rules) at the cost of a round-trip
  and losing the rules from the main thread. Only worth it once the cluster boundaries have
  proven stable.
