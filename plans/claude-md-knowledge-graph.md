# CLAUDE.md as a knowledge graph

Migration roadmap. `CLAUDE.md` becomes a trigger-phrased index; each directive becomes one
node under `docs/rules/` carrying its own evidence and typed edges.

**Status:** `wire/`, `testing/`, `parity/`, `workflow/`, and `style/` clusters migrated. One
cluster (`docs/`) remains inline.

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

## What shipped in the testing/ pass

| Node | From | Status |
|---|---|---|
| `testing/coverage-floor.md` | rule 6 | active |
| `testing/sibling-test-files.md` | rule 8 | active |
| `testing/exercise-production-code.md` | rule 10 | active |
| `testing/derive-from-constants.md` | rule 21 | active |
| `testing/pin-compile-fail-codes.md` | rule 22 | active |
| `testing/clock-seams.md` | rule 26 | active |

These join `testing/fixture-builders.md`, seeded during the `wire/` pass from rule 19's
builder half.

### Rot corrected while migrating

Rules 6, 8, 21, and 26 audited clean — `just cover`'s nightly invocation, the 87 `#[path]`
test wirings with zero inline `mod tests` remaining, `ProtocolFeature { name, min_version }`,
and all four `*_from` helpers verified present. Rule 22 was substantially wrong:

| Was | Is |
|---|---|
| Rule 22's precedent pins `compile_fail,E0639` for "cannot construct a `#[non_exhaustive]` struct externally" | **No such doc-test exists.** The repo's only `compile_fail` is `compile_fail,E0599` at `src/contracts/mod.rs:312`, guarding a *typestate builder* terminal — `Contract::futures("ES").build()` with no month. Wrong code and wrong subject |
| Rule 22: "same logic applies to `trybuild` `compile_fail` files" | There is no `trybuild` dependency |
| Rule 10: `assert_request<B>(builder)` | `assert_request(&message_bus, index, &builder)` — three args; the msg id comes from `B::MSG_ID` via `RequestEncoder` |

The directive in each case survived; only the evidence was wrong. That is the same pattern the
`wire/` audit found, and it is the argument for auditing every cluster before migrating it.

## What shipped in the parity/ pass

| Node | From | Status |
|---|---|---|
| `parity/feature-matrix.md` | rule 1 | active |
| `parity/no-block-on.md` | rule 5 | active |
| `parity/dual-feature-types.md` | rule 12 | active |
| `parity/no-parity-wrappers.md` | rule 13 | active |
| `parity/subscription-consumer-idiom.md` | rule 24 | active |

### Rot corrected while migrating

Rules 5, 12, and 24 audited clean — no `block_on` in `src/`, `server_version_cache: AtomicI32`
present, `NoticeStream`'s `sync_impl` / `async_impl` split and `client::blocking` mirror intact,
`r#async::` paths real, and both tests rule 24 names still exist.

| Was | Is |
|---|---|
| Rule 13: acceptable mirror is "`filter_data` / `filter_data_stream`" | **No `filter_data_stream` method exists.** Both sides spell it `filter_data`; the *return* types differ (`FilterData<I>` vs `FilterDataStream<S>`). `filter_data_stream_drops_notices` is a test name — likely the source of the confusion |
| Rule 1: "default-async, sync-only, and all-features builds must compile and pass tests" | True as a *requirement*, but **nothing enforced sync-only on a PR.** `just test` and `ci.yml` both use `--features sync`, which keeps `default = ["async"]` on — that leg is sync **plus** async. `--all-features` has no CI leg at all |

The rule-1 finding has a documented incident: #658 added an unconditional `#[tokio::main]`
doctest to `lib.rs`, breaking sync-only compilation. Every PR check stayed green; the Coverage
job (`on: push` to `main`) went red and stayed red for 11 commits until #671 fixed it. The
`CLAUDE.md` clippy trio had the same flag bug and is corrected in this PR — its middle config
was sync+async, while the rustdoc trio's middle config was genuinely sync-only.

### Follow-up this pass surfaced — ~~open~~ **closed in the stacked PR**

`just test` and `ci.yml` did not cover sync-only or `--all-features`. Both now run one leg per
configuration, with flags spelled out per leg so the additive-features trap cannot reappear
through a `matrix.feature` interpolation. `docs/build-and-test.md` and `docs/code-style.md`
carried the same misconception in ~16 command lines and were swept.

CI cost: two legs to three, so roughly +50% wall-clock on the matrix job. Judged worth it —
the gap it closes went unnoticed through 11 merges.

## What shipped in the workflow/ pass

| Node | From | Status |
|---|---|---|
| `workflow/pre-pr-checks.md` | rule 3 | active |
| `workflow/pinned-toolchain.md` | rule 7 | active |
| `workflow/modernize-touched-modules.md` | rule 9 | active |
| `workflow/integration-crate-builds.md` | rule 11 | active |
| `workflow/restrict-after-callers.md` | rule 23 | active |

`CLAUDE.md`'s Quick Commands block survives the migration — the commands themselves are worth
keeping in permanent context — but its comment blocks, which had grown into a second copy of
rules 3 and 6, now point at the nodes. It is also split into the unconditional gate and the
situational one (integration builds, `just rules-check`, `just cover`).

### Rot corrected while migrating

Rules 9 and 11 audited clean. Rule 7's version pins are correct (1.95.0 on `main`, 1.93.0 on
`v2-stable`), though it undersold the mechanics — `ci.yml` pins the toolchain in **two** jobs,
and `coverage.yml` / `security.yml` are deliberately unpinned.

| Was | Is |
|---|---|
| Rule 3: "all three clippy configs" — third line was `cargo clippy --all-features` | Weaker than the CI leg it mirrors, which runs `--all-targets ... -- -D warnings`. Corrected in the block |
| Rule 3 implies the rustdoc trio mirrors CI | **CI never sets `RUSTDOCFLAGS`.** `ci.yml` runs bare `cargo doc --no-deps`, so a broken intra-doc link warns and passes. The local trio is the *only* gate; docs.rs is where it otherwise surfaces |
| Rule 23's precedent: #547 → #548 added `#[non_exhaustive]` on `Contract` | #665 **removed it again** — with the fields typed, it was construction friction guarding mistakes that already failed to compile. The caller-first split was still right; the restriction was not. #548's `compile_fail` guard went with it, which is why [pin compile_fail codes](../docs/rules/testing/pin-compile-fail-codes.md) found no such doc-test in the `testing/` pass — same deletion, found from the other end |

The #665 reversal is now the most instructive half of that node: it is a precedent for the
ordering rule *and* a counter-example to the restriction itself, which is the point
`#[non_exhaustive]` is deliberate rather than default.

**Review of this pass caught the graph doing it again.** The rule-9 node first claimed #657 left
"no inline blocks and every test file `#[path]`-wired." Neither half held: `src/messages.rs` had
an inline `#[cfg(test)] mod from_str_tests { ... }`, and about two dozen `mod tests;`
declarations still resolve to `<dir>/tests.rs` — the layout
[sibling test files](../docs/rules/testing/sibling-test-files.md) names as the anti-pattern.
That node's own "the convention is fully applied" line (shipped in the `testing/` pass) had the
same overstatement, and was corrected here. The inline block moved to `src/messages/tests.rs`;
the `<dir>/tests.rs` residue is left to the rule that governs it — convert one when you are
already in the module.

**Lesson for the remaining passes: audit the completeness claims, not just the API names.** Both
audits so far checked that cited symbols exist. Neither checked whether "fully applied", "zero
remaining", or "all N sites" was still true, and that is the class that failed twice.

## What shipped in the style/ pass

| Node | From | Status |
|---|---|---|
| `style/domain-module-layout.md` | rule 2 | active |
| `style/param-budget.md` | rule 4 | active |
| `style/narrow-reexports.md` | rule 14 | active |
| `style/macros-last-resort.md` | rule 25 | active |

Rule 4 was the one rule with a **verbatim second copy** outside `CLAUDE.md` — its two SRP
bullets in `docs/code-style.md` repeated the builder rationale word for word. That copy is now
a one-line pointer at the node. Same shape as the Quick Commands duplication the `workflow/`
pass found, and the reason to grep `docs/` for a rule's text before migrating it.

### Rot corrected while migrating

Rules 14 and 25 audited clean on names and on counts — `src/macros.rs` hosts exactly the two
macros rule 25 names, `impl_str_partial_eq!` still has 3 invocations and `impl_wire_enum!` 8,
both generic demotions (`check_serde_round_trip`, `check_str_partial_eq_round_trip`) survive
as generics, and `orders/common` / `accounts/common` still expose their decoders via
`pub(super) mod` + a narrow `pub(crate) use` list.

| Was | Is |
|---|---|
| Rule 2: "Client methods live in domain modules, not in `client/sync.rs`" | True of 11 domains and **false of four sites**: `Client::order` and `Client::market_data` are defined in `client/sync.rs` and `client/async.rs`, while every sibling builder entry point (`realtime_bars`, `tick_by_tick`, `market_depth`) lives in its domain module. Recorded as known drift on the node |
| Rule 4 implied `#[allow(clippy::too_many_arguments)]` is the canary for the 3-param budget | Clippy's default threshold is **7** — it fires at eight or more, and the budget is three. All four `#[allow]` sites in the tree are 8-arg encoders; a 4-to-7-arg signature passes every gate silently. The budget has no enforcement at all |
| Rule 4's `DateRange { start, end }` | Hypothetical — no such type exists. Kept as illustration, marked as one |
| Rule 25 credits the orphan rule only implicitly ("can't be deduplicated via a blanket trait impl") | `impl_wire_enum!`'s own doc names it outright: `impl<T: WireEnum> Display` is blocked by the orphan rule, which is *why* the macro is the only viable shape. Promoted to the node's first justification |

Two clippy blocks were also still two legs without `--all-targets` — `docs/code-style.md`'s
Linting section and `docs/build-and-test.md`'s Code Quality block. The `parity/` pass swept
`build-and-test.md`'s *pre-PR* block and missed the one 80 lines above it, which is how the same
file ended up contradicting itself. `build-and-test.md` now spells the three legs out once;
`code-style.md` points at [pre-PR checks](../docs/rules/workflow/pre-pr-checks.md) rather than
becoming a third copy.

**Review of this pass caught two fabricated counts in the new nodes.** `param-budget` credited
`PeggedToBenchmark` with "seven defaultable fields" (six setters over eight fields — the number
matched nothing), and `domain-module-layout` said "eleven domains" over a list of ten plus
`client`. Neither came from stale evidence; both were invented while writing prose *about* the
danger of invented counts. Third pass running where a count claim failed, and the first where
the count was new rather than inherited — so the lesson generalises past migration: **any
number in a node needs a command behind it at the moment it is written.**

## Retrieval check — run this before migrating further

**Run 2026-08-06: 3/3 pass. Gate is clear; migration may continue.** Three fresh subagents,
one probe each, no knowledge of the graph design or the expected answers. Every one opened
its node as its *first* read and surfaced the required content. None answered from the index
line alone.

| Probe | Node opened | Surfaced | Verdict |
|---|---|---|---|
| 1 | `wire/proto-aware-accessors.md` | `raw_bytes`-first branch + `text_request_id_field` registration | pass |
| 2 | `wire/enum-typing.md` | verify-wire-first → **no**, cited free-form `"Filled Size: 1"` | pass |
| 3 | `testing/fixture-builders.md` | `src/testdata/builders/<domain>.rs` | pass |

Two findings from the run:

- **This file contaminates its own probes.** Probes 1 and 2 both grepped into it and hit the
  question list *and* the expected answers below. Both read their node first, so the answers
  were node-driven, and probe 1 self-reported the leak — but a re-run must hold the probes
  outside the repo, or the result means nothing.
- **Probe 1 outran its node.** `contract_id` sits at a *varying* tag nested inside a
  `Contract` sub-message (tag 2 for `ContractData` / `OpenOrder` / `ExecutionDetails` /
  `Position`, tag 1 for `PortfolioValue` / `CompletedOrder`, tag 3 for `PositionMulti`), so
  unlike its siblings it needs a `match self.message_type()` and one envelope per tag
  position. The probe also caught a `dead_code` trap the node omits: `ResponseMessage` is
  `pub(crate)` since #581, so a new `pub fn` with no in-crate caller trips `-D warnings`
  (precedent: `is_shutdown`'s `#[cfg_attr(not(feature = "sync"), allow(dead_code))]`). Good
  trigger, thin mechanics — fold both into the node when it is next touched.

The probes, retained for re-runs — three questions, each answerable only from a node:

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

One cluster left:

| Order | Cluster | Rules | Notes |
|---|---|---|---|
| 1 | `docs/` | 18, 27 + Changelog / Release Notes / Maintaining Documentation | Rule 27 already links out to `modernize-touched-modules` and `dual-feature-types`, and `param-budget` defers its `Option<T>`-removal sub-rule to it |

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
- ~~**Stale rule-number citations in source.**~~ **Done in the `testing/` pass.** All 27
  `rule N` citations across `src/` now name node paths instead of numbers. (A 28th,
  `orders/mod.rs:1115`, is IBKR's Rule 80A — a false positive, left alone.)

  **Mapping by number would have been wrong.** The four citations in `market_data/historical/`
  read "rule 19 canary acceptable for builder-fed helpers" — that is the
  `#[allow(clippy::too_many_arguments)]` exception, a **different rule 19** from the migrated
  one (proto fixture migration). Meanwhile ten citations said "rule 20" for what is now
  `proto-only-decoding` (id 15), because they were written under the older numbering, while
  `display_groups/async_tests.rs:72` says "rule 15" for the *same* node under current
  numbering. Old and current schemes coexist in `src/` with no marker distinguishing them, so
  every site had to be read for intent. This is the strongest evidence yet for retiring
  numbers entirely rather than maintaining them.

  The four builder-fed `#[allow(clippy::too_many_arguments)]` sites now cite
  [param budget](../docs/rules/style/param-budget.md), closed in the `style/` pass.
- **Reconcile the maintainer's memory store.** Two `[[wikilink]]` syntaxes coexist for the
  same targets (`[[project-protobuf-only]]` vs `[[project_protobuf_only]]`); the whole
  fixture/builder group sits outside the wikilink graph using backticked filenames; and one
  dangling `[[dual-feature-public-types]]` points at CLAUDE.md rule 12 — it resolves for free
  once rule 12 becomes `parity/dual-feature-types.md`.
- **Consider promoting clusters to subagents.** The node bodies would become the agent
  prompts. Stronger enforcement (the agent always has its rules) at the cost of a round-trip
  and losing the rules from the main thread. Only worth it once the cluster boundaries have
  proven stable.
