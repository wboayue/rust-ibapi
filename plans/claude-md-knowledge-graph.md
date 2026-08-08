# CLAUDE.md as a knowledge graph

Migration roadmap. `CLAUDE.md` becomes a trigger-phrased index; each directive becomes one
node under `docs/rules/` carrying its own evidence and typed edges.

**Status: complete.** All six clusters — `wire/`, `testing/`, `parity/`, `workflow/`, `style/`,
`docs/` — are migrated. `CLAUDE.md` carries no inline rules and rule numbering is retired.
`CLAUDE.md` was 5,012 words when the migration started and is 1,310 now — a 74% cut in what
every session loads, with none of the content lost.

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

### Follow-up this pass surfaced, closed in the stacked PR

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
| Rule 2: "Client methods live in domain modules, not in `client/sync.rs`" | True of 11 domains and **false of four sites**: `Client::order` and `Client::market_data` are defined in `client/sync.rs` and `client/async.rs`, while every sibling builder entry point (`realtime_bars`, `tick_by_tick`, `market_depth`) lives in its domain module. Recorded as known drift on the node. **Closed in #729** — all four sites moved, drift section deleted |
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

## What shipped in the docs/ pass

| Node | From | Status |
|---|---|---|
| `docs/public-api-examples.md` | rule 18 | active |
| `docs/doc-parity-audit.md` | rule 27 | active |
| `docs/changelog-entry.md` | the Changelog section | active |
| `docs/release-notes.md` | the Release Notes Guidelines section | active |
| `docs/user-docs-sync.md` | the Maintaining Documentation section | active |

`style/param-budget` deferred its `Option<T>`-removal sub-rule to "rule 27, still inline in
`CLAUDE.md`" one pass ago; that sentence became false here and now points at
`doc-parity-audit`. Worth noting as a mechanic: a node written during migration that defers to
an unmigrated rule carries a dangling reference until the target lands, and nothing but reading
it catches that.

Three `CLAUDE.md` *sections* migrated alongside the two numbered rules — they were rules in
everything but numbering, and each is situational (you need the changelog rule when shipping a
user-facing change, not on every turn). With them gone, `Key Points to Remember` is empty and
deleted, and the retired-numbers paragraph is replaced by the standing warning that a "rule N"
citation must be resolved against the `CLAUDE.md` of its own date.

### Rot corrected while migrating

| Was | Is |
|---|---|
| Rule 18: "every `pub fn` gets a `# Examples` block" | Holds for **132 of 152** `impl Client` methods after this PR's fixes. Nine sites still missing across seven methods (`exercise_options`, `market_rule`, `family_codes`, `server_time_millis` ×2, `cancel_historical_ticks` ×2, `cancel_contract_details`, async `market_data`), one of them a parity miss — async `market_data` has no example while its sync twin does. Inventoried in `plans/code-consistency-followups.md`; eleven exempt accessors account for the rest |
| `Client::check_server_version` counted as an exempt accessor | Neither exempt nor an accessor — it takes two args, returns `Result`, and was **`pub` on async while the sync twin was `pub(crate)`**. A public-surface asymmetry the doc audit walked past because it was reading the docs, not the visibility. Narrowed to `pub(crate)` here; no callers outside `src/` |
| Rule 18 implied the heading is the block | `head_timestamp` had a runnable example with **no `# Examples` heading** — it compiles under `cargo test --doc` but docs.rs renders no Examples section, and its async twin has the heading. Fixed here, along with two `# Example` singulars on `Client::order` and one `//` typo inside a doc fence |
| Rule 27's precedent: #573 split `historical_schedule` into `historical_schedules` + `historical_schedules_ending_now` | **Neither method exists.** Step 3 of the magic-`None` evolution shipped: the surface is now the builder `historical_schedules(&contract, duration).fetch()`. Test and example *names* still say `..._ending_now`. This is the rule's own prediction coming true, so the node keeps the precedent and says how it ended |
| Release Notes Guidelines listed five formatting rules | Missing the one thing the notes carry that the changelog does not — **contributor attribution**. v3.2.0 has both forms (`Thanks to @bebop23 for the contribution.`, `Thanks to @thimo-seidel for the report.`) and the convention lived only in the maintainer's memory store. Now in the node |

`CHANGELOG.md` audited clean against every claim in the Changelog section: group order, the
PR-number suffix, `## [Unreleased]` at the top, newest-first version sections, and the
link-reference definitions at the bottom. The v3.3.0 release notes match their format too,
except that the H3 heading takes multiple PR numbers (`(#707, #708)`) where the rule showed one
— the node now allows both.

## Retrieval check

Was the gate on migrating each further cluster. With the migration finished it becomes the
regression check: re-run it after a round of node edits, or whenever a node's trigger phrasing
is rewritten.

**Run 2026-08-06: 3/3 pass.** Three fresh subagents,
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
probes ⇒ the trigger phrasing is too weak. One session is noisy evidence; re-run if the result
is ambiguous.

## What the six audits found, in aggregate

Every cluster was audited before migrating, and every cluster but one had rot. The pattern
across all six:

- **Names decay quietly.** `peek_string`, `filter_data_stream`,
  `historical_schedules_ending_now` — each cited confidently, none existing.
- **Completeness claims decay faster.** "Fully applied", "zero remaining", "every `pub fn`" was
  wrong in `sibling-test-files` (two dozen `<dir>/tests.rs` left), `domain-module-layout` (four
  `client/` methods), and `public-api-examples` (25 of 153). Checking that a cited symbol exists
  does not check this class; it has to be counted.
- **The directive almost always survived.** In six passes, no rule was found to be wrong about
  what to do — only about what the code looked like, which gate enforced it, or how a precedent
  ended. That is the argument for the split: the directive is stable enough to index, the
  evidence is not.
- **Enforcement claims were the most dangerous.** Rule 1's sync-only build, rule 3's rustdoc
  trio, rule 4's `too_many_arguments` "canary" — three separate rules implied a gate that did
  not exist. Two are now real CI legs; the third is documented as ungated.

**Numbering is retired, not renumbered.** It was retired in one step at the end rather than
compacted along the way, because the numbers were already unreliable across stores — one memory
cites "rules 20/22" for the family now at `proto-only-decoding` / `proto-aware-accessors`, and
another references a *different* rule 19. Any surviving "rule N" citation has to be resolved
against the `CLAUDE.md` of its own date. Both `plans/` files that were organised by number now
cite nodes.

## Prose replaced by gates — #730, #731

`CLAUDE.md` carried two "traps that pass CI silently". Both are now enforced by code, and the
standing warning is gone from the file.

### The registration trap — #730

The guard is `debug_assert_request_id_routable`
(`src/subscriptions/common.rs`) runs in both subscription constructors and panics when a
`request_id`-keyed subscription is built for a decoder declaring a response type the
dispatcher cannot route to it. Classification is `first_unroutable_by_request_id`
(`src/transport/routing.rs`); it accepts on the same three arms `determine_routing` uses.

The placement is what makes it work. `MessageBusStub` tests bypass the dispatcher — that is
the whole reason the failure was invisible — but they do not bypass the constructor, so the
guard fires in exactly the tests that used to pass in silence. Compiled out of release builds;
the invariant is over static tables and cannot depend on caller input.

**It found a live bug on its first run.** `MarketDataType` has been in `TickTypes`'s
`RESPONSE_MESSAGE_IDS` since #516 (2026-05-05) and never in `text_request_id_field`, so
`TickTypes::MarketDataType` could not reach a `market_data` subscription — the tick was routed
to a shared channel nobody subscribes to and dropped. Nine tests failed the moment the guard
went in. The decoder had a unit test, and it passed: it calls `TickTypes::decode` directly,
which is the same one-layer-too-low seam the rule warns about.

Generalising past this file: **a rule that names its own silent-failure mode is a rule with a
missing gate.** The rule entered `CLAUDE.md` with #647 on 2026-05-27, three weeks *after* #516
put the second instance in the tree. It described that failure accurately for ten weeks and
caught none of it.

### The fixture-framing trap — #731

The second trap looked harder, and the #730 write-up said so: a text-framed fixture reaching a
proto-only decoder is skip-classified, so the test goes green with its assertions unrun — but
`Error::UnexpectedResponse` legitimately means "not my message" on shared channels, and
several tests assert that skip on purpose.

**The two failures were never the same thing; one variant was doing both jobs.** Splitting
them by *call site* separates them cleanly:

| Call site | Meaning | Disposition |
|---|---|---|
| `_ => Err(Error::unexpected_response(message))` | not my message type | `Skip` |
| `message.require_proto()?` | my message type, unreadable framing | `Error` |

So `require_proto()` now returns a new `Error::UnexpectedWireFormat`, which
`process_decode_result` does not skip. `Error` is `#[non_exhaustive]`, so the variant is
additive. The trap now costs a red test instead of a silent one; production behaviour is
unchanged in practice, since at floor 213 every message with a proto decoder arrives
proto-framed.

The sweep was 40 tests, and their names did the sorting: every one of them was already called
`*_rejects_text_framing`, so the failing set *was* the set to update — no judgement call per
site. The single exception is the tell: `test_decode_realtime_bar_text_arrival_skip_classifies`
was named for the behaviour being removed, and it was the only test whose name had to change.
**When a rename sweep is driven by the test names themselves, the one test that doesn't fit
the pattern is the one encoding the old contract.**

The lesson generalises past this instance: the #730 write-up recorded this trap as *resistant*
to gating because two cases shared an error variant. That framing was the obstacle. The
question worth asking first is not "how do I detect the bad case" but "why are these two cases
the same value" — and the answer was that they never should have been.

### The mechanism underneath both — #732

`/simplify` on #731 found that the two traps were instances, not the defect. `Result<T, Error>`
was a three-way channel: "skip me" travelled in-band as `Error::UnexpectedResponse`, a variant
~20 one-shot call sites also return *to the user* as a genuine error. Any decoder that reused
it inherited "silently drop this" without asking. #508 was the first patch, #731 the second,
so the rule of three tripped.

The fix that suggested itself — `decode -> Result<Option<T>, Error>` — would have touched all
28 decoder impls. **The better answer was already in the tree.** `RESPONSE_MESSAGE_IDS` had
been sitting on `StreamDecoder` since before this arc, `#[allow(dead_code)]` until #730 made
it load-bearing for the routing guard. It is exactly the question the drivers were asking the
error type: *is this message mine?* So both drivers now filter on it before calling `decode`,
`ProcessingResult::Skip` is gone, and no error variant carries dispatch semantics.

**The reason to prefer the const is not that it touches fewer files** — that was the first
justification written here, and `/simplify` was right to call it the weakest available one.
The real reason is that a const is *statically inspectable* and a return value is not.
`debug_assert_request_id_routable` reads `RESPONSE_MESSAGE_IDS` at subscription-build time to
catch the #647/#730 routing gap; encode the same answer in `decode`'s return type and that
guard has nothing to read, so you retire one silent-failure class by reopening another.
Dropping the const's default (`= &[]`) is the same property — "declare or don't compile" is
enforceable on a const and not on a return shape. That mattered immediately: nine test-side
fake decoders failed to compile and had to declare what they consume.

**The risk moved rather than vanished, and only half of it was bounded at first.** Skip is now
driven by a hand-maintained list. Declared-but-unhandled is caught loudly by the `_ =>`
backstop. Handled-but-undeclared was not, and it is the worse direction: the arm is unreachable
and data vanishes quietly. The first write-up of this section claimed "every domain's stub
tests feed their types through a real subscription" — `/simplify` checked, and it is false for
`contracts`, `market_data/realtime`, and `wsh`, which call `decode` directly. So the suite
passing unchanged was good evidence for 5 of 8 domains, not all of them.

What genuinely improves is the *trigger*. The old mistake was action-at-a-distance — reuse a
variant, inherit a disposition you never knew you were choosing. The new one is local and
visible: a line missing from a const ten lines above the match you are editing. Likelihood
drops even though severity doesn't. The remaining gap is now closed too — the first follow-up
below shipped, and both directions are gated by
`src/subscriptions/response_message_ids_tests.rs`.

**The reusable lesson: when a fix needs a fact, check whether the codebase already declares
it.** Three PRs in a row here found the answer in `RESPONSE_MESSAGE_IDS` — #730 read it for the
routing guard, #732 for the skip filter — a constant that had been dead code for most of its
life.

## The maintenance protocol is load-bearing now

The founding motivation for this migration was that **the rules rotted silently** — nothing
compile-checks `CLAUDE.md`, and every one of the six cluster audits found decay. `just
rules-check` closed the structural half (links, ids, `related`, `@`-imports, line-number
citations) but validates nothing about whether a node is still *true*, which is the half that
actually decayed.

That gap was covered only by prose in `docs/rules/README.md` — a file that loads on demand,
i.e. exactly when you are already thinking about the graph, and never when you are deleting a
function some node happens to cite. The obligations now live in
[Maintaining the rule graph](../CLAUDE.md#maintaining-the-rule-graph) in permanent context, and
`docs/rules/README.md` keeps the mechanics and points up rather than restating them.

Cost: `CLAUDE.md` goes 1,256 → 1,558 words. Against the 5,012 the migration started from, that
is still a 69% cut, and it buys the one thing the index could not previously do — fire on
*writing* a claim rather than on reading one.

What made it into permanent context is what the audits proved, not everything that could be
said: retire-don't-delete, grep the graph when renaming, put a command behind every count,
append precedents including the ones that became counter-examples, and treat a node describing
its own silent failure as a missing gate. The counts rule earns its line twice over — two
fabricated counts shipped in the `style/` pass in prose *about* fabricated counts, and the
`#734` follow-up bullet in this file got both its numbers wrong (41 vs 78 entries, eight vs 16
decoders).

## Follow-ups

- ~~**Cross-check the two lists so `RESPONSE_MESSAGE_IDS` cannot drift from the `decode`
  arms.**~~ **Shipped** in `src/subscriptions/response_message_ids_tests.rs`. The probe turned
  out to be an exact biconditional rather than the one-directional assert the plan sketched:
  with a *text*-framed minimal frame, `UnexpectedResponse` means "no arm" and everything else
  means "arm exists", because every real arm either returns without touching the payload or
  reaches `require_proto()` and raises `UnexpectedWireFormat` (#731). So declared-but-unhandled
  and handled-but-undeclared are both caught by the same probe, and the second test direction
  cost nothing. The `stream_decoder!` macro alternative stayed unbuilt, as planned.

  The hand-listed roster cost was real and is itself gated: `test_decoder_roster_is_complete`
  counts `impl StreamDecoder` blocks under `src/` and fails when the roster falls behind. That
  is the completeness-claim class the six audits found decays fastest, so it gets a command
  behind it rather than a comment.

- ~~**Unify the third driver.**~~ **Shipped in #738.** `TickDecoder::MESSAGE_TYPE` became
  `RESPONSE_MESSAGE_IDS: &'static [IncomingMessages]` and `classify` now filters with the same
  `is_undeclared` helper the two subscription drivers use. A tick type declares one entry; the
  arity was the only difference, and spelling it as a list made that visible rather than
  structural.

- ~~**Audit the 41 `RESPONSE_MESSAGE_IDS` entries against the const's new meaning.**~~
  **Shipped.** Both numbers in the original bullet were wrong, and the audit is what caught
  them: there are **78** declared entries, not 41, and **16** decoders declared
  `IncomingMessages::Error`, not eight. Counting them took one throwaway test over the roster
  #733 had already built — which is the argument for building the roster, and one more instance
  of the count class this file keeps failing.

  The finding itself held. `determine_routing` classifies `Error` before the allow-list, so it
  arrives as `RoutedItem::Error`/`Notice` and never as the `Response` that `decode` consumes.
  All 16 declarations and their `decode` arms are gone, and `routable_to_request_id_subscription`
  lost the `Error` exemption that had been keeping them legal — the circle is broken from the
  const side, not the guard side. `test_response_message_ids_match_decode_arms` now names the
  mistake specifically for `Error` and `Shutdown` rather than reporting it as a generic
  declared-but-unhandled failure.

  **The proof generalises further than the const.** The one-shot request path reads through
  `RoutedItem::into_legacy`, which maps an error to `Some(Err(_))` and never runs the
  processor — so every `IncomingMessages::Error` arm in a one-shot `decode_*_message` is dead
  too. Two of those (`wsh::decode_metadata_message`, `decode_event_data_message`) are shared
  with a `StreamDecoder` and were removed here; the rest are a separate follow-up below.

  Removing the arms cost 15 decoder-level `test_decode_error_message*` tests, all asserting a
  path only `MessageBusStub` can produce. The behaviour they claimed to cover is tested at both
  layers that actually implement it — `test_hard_error_with_request_id_terminates_subscription`
  and `test_subscription_hard_error_terminates_stream`, in each transport's tests.

- ~~**Retire the remaining one-shot `IncomingMessages::Error` arms.**~~ ~~**Make
  `MessageBusStub` classify like the dispatcher.**~~ **Both shipped in #735, and they turned
  out to be one job.** Thirteen dead sites went: seven `decode_*_message` dispatchers
  (`accounts` ×3, `contracts`, `scanner`, `config` ×2) and six inline
  `message.message_type() == IncomingMessages::Error` guards in `contracts::{sync,async}` and
  `market_data::historical::{sync,async}`. Kept: `connection/common.rs`, which reads frames
  during the handshake before a dispatcher exists, and `messages/parser_registry.rs`, which is
  a trace-parser table.

  **Deleting the arms was going to cost a third batch of tests, and that was the signal to stop
  deleting.** #734 dropped 17 tests because only `MessageBusStub` could produce their input;
  two more were about to go the same way. The rule of three tripped, so the stub was fixed
  instead: `routed_items()` runs every fixture through `determine_routing`/`classify_error`, so
  an error frame arrives as `RoutedItem::Error`/`Notice` exactly as on the wire. Both tests then
  passed unmodified. **Fixing the fixture kept the coverage that deleting the arms would have
  destroyed** — and the two tests #734 deleted on the same grounds would have survived it too.

  The feared tail — warning and data-advisory codes becoming `Notice` and being filtered —
  never materialised: one test needed touching across both legs, and only because its assertion
  encoded the old wart.

  **One dead arm was hiding a live bug.** Blocking `matching_symbols` read its response with
  `if let Some(Ok(mut message))`, so a routed error fell through to `Ok(Vec::new())` — a
  rejected pattern was indistinguishable from "no symbols matched". Its async twin already had
  `Some(Err(e)) => return Err(e)`. The dead `IncomingMessages::Error` arm sitting next to the
  hole is what makes it legible: someone meant to handle errors and wired it one layer too low,
  and the arm's unreachability is exactly why the omission below it stayed invisible.
  **A dead arm is worth reading before deleting — it marks where someone expected a case to
  arrive, which is where to check whether the real case is handled at all.**

  **The #734 deletions were wrong in principle, not just in outcome.** The justification was
  "the mechanism is covered at the transport layer", which conflates *delivery* with
  *consumption*. The transport tests prove the dispatcher produces `RoutedItem::Error`; they say
  nothing about whether a given public API hands it to the caller — and two APIs did not. Both
  deleted tests were restored and passed unmodified against the classifying stub.

  A sweep of every `Some(Ok(..))` consumption site for the same shape found one more:
  `OrderBuilder::analyze()` dropped routed errors on both sides (`if let Ok(..)` inside the
  loop on sync, `while let Some(Ok(..))` on async), returning `UnexpectedEndOfStream` instead of
  the rejection — on the one API where rejection is a routine outcome. Every other site handles
  `Some(Err(e))`. **The per-API question "does this consume `Err`?" needs asking once per
  public entry point; no shared layer answers it.**

- ~~**Adopt `fold_one_shot` at the ~14 sites that hand-roll it.**~~ ~~**Collapse the seven
  identical `decode_*_message` dispatchers.**~~ **Both shipped in #736**, which the follow-up
  list here did not record — the entries survived two more passes describing work already done,
  which is the same rot the six audits kept finding, one level up. **A follow-up list is a
  completeness claim too, and nothing gates it.** The residue each left is closed in #738: the
  four option-computation sites, called "genuinely blocked" because they need `&mut` plus a
  `DecoderContext`, now use a `fold_one_shot_mut` that `fold_one_shot` itself delegates to, so
  the `Some(Err)` / `None` disposition is one decision rather than two.

- ~~**Six one-shot folds are still hand-rolled.**~~ ~~**Make the expected message type a helper
  parameter (`one_shot_typed`).**~~ **Both shipped in #738, and they were one job.** The helper
  is `expect_proto(expected, decode_proto)` — a combinator returning the `impl Fn(&ResponseMessage)`
  the one-shot helpers already take, rather than a new parameter on three functions that carry
  six. Keeping the arity fixed matters more than it looks: these helpers are already three over
  the [param budget](../docs/rules/style/param-budget.md), and the budget has no gate.

  **The count in the original bullet was wrong in the safe direction, which is the rarer
  failure.** It said four one-shot sites do not narrow at all. The real number is 26 sites over
  13 distinct decoders — every bare `decoders::decode_x` passed as a processor, not just the
  four that were named. Command:
  `grep -rn "expect_proto(IncomingMessages::" --include=*.rs src/ | grep -v -E "_tests\.rs|/tests\.rs" | wc -l`
  → 50 call sites: 20 replaced a `decode_*_message` wrapper, 26 replaced no narrowing at all,
  and 4 replaced an inline `if message.message_type() == ..` guard. Cross-checked by
  `SITES_PER_PAIR` in `src/common/one_shot_pairing_tests.rs`: 25 pairs × 2 (sync, async) = 50.

  **The first draft of this bullet cited `grep -v _tests`, which returns 56, not the 44 it
  claimed.** Four test files are named `tests.rs` with no underscore, so the filter missed them.
  Written inside the bullet arguing that a follow-up list is an ungated completeness claim.
  Fifth instance in this file of a count that was not run; the fix each time is the same and it
  keeps not being applied at the moment of writing.

  Narrowing is now structural: a one-shot call site cannot name a payload decoder without also
  naming the frame it belongs to. 30 decoder functions went with it — 8 `decode_*_message`
  dispatchers and 22 message-level `require_proto()` wrappers that existed only to be their
  bodies. The two wsh wrappers stay: they are shared with a `StreamDecoder`, where the expected
  set is already `RESPONSE_MESSAGE_IDS`.

  **The narrowing found two test fixtures that had been lying, and the class generalises.**
  `test_decode_market_rule_rejects_text_framing` built its frame with message id 87; `MarketRule`
  is 93. `test_decode_news_providers_rejects_text_framing` led with the literal string
  `"newsProviders"`, which parses as no discriminant at all. Both passed for as long as they
  existed, because the only assertion was that `require_proto()` rejects text framing and the
  type prefix was never read. **A fixture field that no assertion depends on is unverified
  input, and it will be wrong.** Two more turned up the same way in the same PR (histogram 89 was
  right, market-depth-exchanges 71 should have been 80).

  One hand-rolled fold survives on purpose: `historical_data`'s fetch reads *two* frames (the
  data message, then `HistoricalDataEnd`), so it is not a one-shot at all.

  **`/simplify` found the sweep had missed wsh and `next_valid_order_id`, and that the "44 sites
  now narrow structurally" claim was false while they existed.** `next_valid_order_id` is the
  instructive one: it reads the shared `RequestIds` channel through `fold_one_shot` with a bare
  `decode_next_valid_id`, i.e. exactly the bug class, in the file that had already adopted
  `fold_one_shot`. Adopting the fold helper reads as adopting the convention; it isn't the same
  thing, and nothing distinguished them until the roster existed.

  **The bigger finding was that nothing gated the pair at all.** A review mutated three sites to
  mismatched `(IncomingMessages, decoder)` pairs and only incidental per-API round-trip tests
  failed — the same silent-failure shape the const's own gate was built for. Closed by
  `test_expect_proto_sites_match_the_roster` (`src/common/one_shot_pairing_tests.rs`): every site
  scraped from `src/`, checked against a `PAIRS` roster in both directions, with each pair
  required to appear exactly twice so sync and async cannot drift. The convention now has a node,
  [one-shot narrowing](../docs/rules/wire/one-shot-narrowing.md) — it had shipped as a doc comment
  on a `pub(crate)` fn, which `just rules-check` cannot see because it validates structure, not
  coverage.

  **And `/simplify` was wrong about one thing, loudly and usefully.** Three of its four agents
  independently said the `expect_type` inside the two single-type `StreamDecoder::decode` impls
  (scanner, wsh) was now a redundant third copy of `RESPONSE_MESSAGE_IDS`, quoting the new
  `expect_proto` doc back at it. Removing them turned
  `test_response_message_ids_match_decode_arms` red within one run: a decoder with no match has
  no `_ =>` arm, so `expect_type` *is* its backstop, and without it the probe reads the decoder
  as claiming an arm for every message type including `Shutdown`. **The doc comment they were
  quoting was mine and it was the actual defect** — it said "not for `StreamDecoder`" without
  saying why, so it read as "narrowing there is redundant." Recorded on the node as a
  counter-example. Convergent agreement across independent reviewers is not evidence; the gate is.

- **The order-builder tests re-implement the code they test.** `src/orders/builder/{sync,async}_impl/tests.rs`
  define their own `analyze` / `submit` on `OrderBuilder<'a, MockOrderClient>` returning
  `Vec<PlaceOrder>` rather than a `Subscription`, so the production methods on `Client` had zero
  coverage — which is why the `analyze` bug above survived four tests named for it. #735 added
  two tests at the real seam for `analyze` only; `submit`, `build_order`, and the bracket-order
  builders still have none. The async shadow carried the very discard the PR fixed
  (`while let Some(Ok(..))`) and was corrected in place, which is the argument for deleting the
  shadows rather than maintaining two copies.

  **The `analyze` half is closed.** Both shadow `analyze` impls and their four tests are gone;
  the coverage moved to `orders/{sync,async}_tests.rs` against `Client::stubbed`, where the
  happy path, the empty-stream path, and the rejection path all run the production method.
  Removing that one shadow made the entire mock `place_order` path dead — field, setter, and
  both client methods — which is itself evidence of how much of the mock existed only to feed a
  duplicate.

  **What remains, and why it is not mechanical.** The `submit` / `submit_all` /
  `submit_oca_orders` / `submit_with_updates` shadows are still there. `submit_all` and
  `submit_oca_orders` carry real logic (id reservation, `parent_id` wiring, transmit flags), so
  they are the drift-prone ones worth doing next. Converting them means asserting on the
  captured `PlaceOrderRequest` proto via `decode_request_proto` rather than on a decoded `Order`
  struct — a better assertion, but a rewrite per test, not a substitution. One test cannot move
  at all: `test_order_submit_with_error` injects a failure from `submit_order`, which
  `MessageBusStub` has no way to produce, so it is testing the mock's error injection rather
  than any production path. A textbook
  [exercise production code](../docs/rules/testing/exercise-production-code.md) violation, and
  the largest one left in the tree.

- **Cache the message discriminant on `ResponseMessage`.** `message_type()` re-parses
  `fields[0]` with `i32::from_str` on every call, and it is called 4–6 times per inbound
  message (routing, `request_id`, the new filter, the decoder's own match). Worse,
  `from_protobuf` does `message_type.to_string()` per message purely so the discriminant can be
  re-parsed downstream — `fields[0]` has no other reader. Storing `kind: IncomingMessages` at
  construction makes the accessor a field read and removes an allocation per message. Pure
  perf, no behaviour change.

- **Collapse the 40 `*_rejects_text_framing` asserts into one helper.** They spell the same
  assertion four different ways across 12 files, with four different panic messages. #731 is
  the evidence: a pure variant rename touched all 40 sites and produced ~600 of its ~700
  test-side lines; with a helper it would have been one line. The precedent is
  `assert_decimal_parse_error` in `src/common/test_utils.rs` — same shape, 33 call sites, and
  its doc states this exact rationale. Not speculative infra: the consumers exist today, and
  the three largest files already import from that module. Deferred out of #731 as
  restructuring, per [/simplify scope discipline](../CLAUDE.md).

- ~~**Gate the three `TickDecoder::RESPONSE_MESSAGE_IDS` consts.**~~ **Shipped in #739.** The
  review's mutation reproduced first — `TickDecoder<TickBidAsk>` declaring
  `[HistoricalTickBidAsk, UserInfo]` passed all 1364 sync tests (the bullet said 1384; the count
  had drifted, as they do) — and now fails by name in both directions.

  **The precondition was the whole job.** The gate could not be pointed at `TickDecoder` as it
  stood: with no backstop, all three `decode` impls dive straight into `require_proto()` and
  answer `UnexpectedWireFormat` to every probe, so the check would have read them as handling all
  88 scanned discriminants and asserted nothing. Adding `expect_type` first is what made them
  legible. #738 had learned the same fact from the opposite end — it deleted two `expect_type`
  narrows from single-type `StreamDecoder` impls as redundant and the gate went red within the
  hour. **A backstop is not defensive coding; it is the thing that makes a decoder's arm set
  observable from outside.**

  The two traits now share `check_decoder`, with the differing `decode` signatures erased by a
  closure at the two call sites — the failure taxonomy stays in one copy rather than growing a
  second that drifts. `test_decoder_roster_is_complete` counts per trait, so adding a
  `TickDecoder` while deleting a `StreamDecoder` cannot net out to a passing total.

- **Retire the `PAIRS` roster with a `ProtoPayload` trait.** `trait ProtoPayload { const
  MESSAGE_ID: IncomingMessages; fn decode(bytes: &[u8]) -> Result<Self, Error>; }` implemented
  once per payload makes `expect_proto::<T>()` take zero literals, moves the pair to where it is
  a property (the proto type, not the caller), deletes the hand-listed roster *and* the
  sync/async double-spelling in one move, and makes the population enumerable the way
  `check_all` enumerates `StreamDecoder`. 25 impls plus a signature change on three helpers —
  restructuring, deferred out of #738's `/simplify` per
  [scope discipline](../CLAUDE.md#maintaining-the-rule-graph).

- ~~**Widen the one-shot helpers to `&mut ResponseMessage` and delete `fold_one_shot_mut`.**~~
  **Shipped in #740 + #741, by narrowing rather than widening.** The bullet assumed the `&mut` was
  load-bearing. It was vestigial: `StreamDecoder::decode` had carried it since the text era, and
  every one of the ~25 decoder functions took the message mutably and immediately called
  `require_proto()`, which takes `&self`. The library compiled in all three feature configs on the
  signature change alone — no body needed touching, which is the proof. The only production code
  still advancing a cursor is the handshake, which never goes through `StreamDecoder`:
  `grep -rn "\.next_int()\|\.next_string()\|\.next_double()" --include=*.rs src/ | grep -v _tests`
  → two hits, both in `connection/common.rs`.

  So the four option-computation sites did not need a wider helper; they needed a decoder that
  told the truth about what it reads. `fold_one_shot_mut` and the `From<&mut ResponseMessage> for
  Notice` coercion shim both existed only to serve that `&mut`, and both are gone.

  **The lesson is about how the bullet was written.** "Widen the helper" named a fix for a
  constraint nobody had checked was real. One grep would have inverted it, and the same grep was
  available when the bullet was written. Third instance in this file of a claim that needed a
  command and did not get one — the first two were counts, this one was a constraint.

- ~~**Decide which one-shots should retry.**~~ **Shipped in #741: all of them, and the question
  was malformed.** The counts held exactly — 44 retrying, 4 non-retrying, 6 hand-rolled — but
  "by choice" did not. Nobody chose; the helper chose — the mechanism, and the two-axes lesson it
  generalises to, are on
  [one-shot narrowing](../docs/rules/wire/one-shot-narrowing.md#why-every-one-shot-retries).

  What belongs here is what the node cannot carry: the bullet asked for a *rationale to write
  down*, and there was none to find. Every framing it offered — "decide which should retry",
  "nothing says why `head_timestamp` retries and `market_rule` does not" — presumed a decision had
  been made. Checking whether that was true came before answering it, and inverted the task.

  Untested, and worth saying so: `MessageBusStub` cannot produce `Error::ConnectionReset` — only
  the real transport's reconnect path emits it — so no per-API test covers retry wiring, for the
  10 sites migrated here or the 44 that already retried. The combinator itself is covered in
  `src/common/retry_tests.rs`. Closing that is a follow-up below. Command for the total:
  `grep -rn "one_shot_with_retry(\|one_shot_request_with_retry(" --include=*.rs src/ | grep -v -E "_tests\.rs|/tests\.rs" | grep -v "pub \(async \)\?fn" | wc -l`
  → 54, which is 44 + 4 + 6 and the sum the three pre-#741 counts were always claiming.

  **The count in the first draft of this bullet said 49, and nothing produced it.** It was
  written in the same paragraph as the sentence admitting the retry path is untested — so the
  claim that got checked was the one about coverage, and the arithmetic beside it went out
  unchecked. Fourth instance in this file. The next
  author picks a helper by copying a neighbour.

- **Give `on_none` a default and get both one-shot helpers under the param budget.** 42 of the 54
  sites pass the identical `|| Err(Error::UnexpectedEndOfStream)`; of the rest, 6 pass
  `|| Ok(Vec::new())` and 4 pass `|| Ok(Vec::default())` — the same value spelled two ways, which
  is the tell that nobody is choosing here either. Defaulting it and adding an `_or_else` variant
  for the dozen collection sites lands the helpers at 3 and 2 parameters, under
  [param budget](../docs/rules/style/param-budget.md) for the first time, and deletes ~42 closures.
  The node currently argues that `expect_proto` had to be a combinator *because* the helpers are
  over budget; this removes the premise. 54 sites, so it is restructuring.

- **Rename the helpers now that `_with_retry` distinguishes nothing.** With the non-retrying
  helper gone, both names carry a suffix no longer contrasting with anything, while the axis that
  does distinguish them — shared channel vs request id — is not in either name.
  `one_shot_shared` / `one_shot_by_request_id` say it. 54 call sites; worth folding into the
  `on_none` change rather than doing separately.

- ~~**`historical_data` still hand-rolls its retry, and the two sides have already drifted.**~~
  **Shipped in #744.** The drift was not cosmetic: async retried the *wrong* case. A routed
  `ConnectionReset` arrived through `Some(Err(e))` and returned to the caller on the first
  occurrence, while a closed stream — which no retry can fix — was re-sent five times and then
  reported as `ConnectionReset`. Sync had it right. Both now call `retry_on_connection_reset`,
  which also drops the attempt count from `MAX_RETRIES = 5` to the `DEFAULT_MAX_RETRIES = 3` every
  other request uses; `MAX_RETRIES` had no other reader and is deleted.

  **The two tests that should have caught it were the clearest statement of the bug.** Both were
  named `test_historical_data_connection_reset_after_retries`, and they asserted opposite
  outcomes for the same input — `UnexpectedEndOfStream` on sync, `ConnectionReset` after 5 sends
  on async — each with a comment explaining why its side was correct. **Two sibling tests with
  the same name and contradictory assertions are a drift report, not coverage.** Reading a
  sync/async pair side by side is the check; nothing automated compares them.

  The reset case now has a real test on both sides, which #743 is what made possible. The second
  read (the `HistoricalDataEnd` frame) also dropped `Some(Err)` on both sides — the exact shape
  #735 swept for — and now propagates.

- ~~**Teach `MessageBusStub` to inject `Error::ConnectionReset`.**~~ **Shipped in #743.**
  `with_connection_resets(n)` answers the first `n` requests with a bare
  `RoutedItem::Error(Error::ConnectionReset)` and no responses, which is what the socket dropping
  actually looks like; the assertion is the resend count, as the bullet predicted. Four tests
  through `server_time` on both sides cover the two outcomes — under the limit it re-sends and
  succeeds, past it the reset surfaces — and a mutation (retry limit 0) reproduces red before the
  fix, which is the only evidence that matters for a test claiming to gate something.

  **The coverage is one site, not 54, and saying otherwise would repeat the mistake this file
  keeps recording.** `server_time` stands in for the roster; nothing checks that the other 53
  sites call a retrying helper rather than hand-rolling. What changed is that the capability
  exists, so the next API that needs it has somewhere to assert.

  Precursor in #742: 55 of the stub's 56 struct-literal construction sites became
  `with_responses` / `with_ordered_responses` calls. A field costs one edit now rather than 56 —
  and the reason the field was cheap to add is the reason it had never been added.

  The bullet said "49 one-shots" where the bullet above it says 54 and produces the command for
  it. Sixth instance in this file, and the first where the two numbers were in adjacent bullets.

- **Re-audit on a cadence, counting the completeness claims.** All six clusters were audited
  once; what decays fastest is "fully applied" / "zero remaining" / "every `pub fn`", and
  checking that a cited symbol exists does not check it. The two live counts are 134/152
  `# Examples` and the `<dir>/tests.rs` residue.
- **Close the two inventories the audits opened.** Eight missing `# Examples` and the
  `param-budget` violations both sit in
  [plans/code-consistency-followups.md](code-consistency-followups.md), and both are
  take-one-when-you-are-in-the-file work rather than sweeps.
- **Finish the error-classification audit.** #731's write-up named three tables encoding the
  same variant→disposition decision — `process_decode_result`, `is_transient_error`,
  `categorize_error` — and #732's first draft deleted that bullet having fixed one. `/simplify`
  caught it; `is_transient_error` and `categorize_error` are corrected here (both now treat
  `UnexpectedResponse` as fatal, since it means "decoder bug" rather than "stray frame"), but
  the underlying point stands: nothing enforces that a new `Error` variant gets classified in
  all of them, and both those functions are dead code with `#![allow(dead_code)]`. Decide
  whether they have a future or should be deleted.

- **Reconcile the maintainer's memory store.** Two `[[wikilink]]` syntaxes coexist for the
  same targets (`[[project-protobuf-only]]` vs `[[project_protobuf_only]]`); the whole
  fixture/builder group sits outside the wikilink graph using backticked filenames; and one
  dangling `[[dual-feature-public-types]]` points at CLAUDE.md rule 12 — now resolvable, since
  rule 12 is `parity/dual-feature-types.md`. The release-notes attribution convention that
  lived only in that store is now in [release notes](../docs/rules/docs/release-notes.md);
  sweep for others like it.
- **Consider promoting clusters to subagents.** The node bodies would become the agent
  prompts. Stronger enforcement (the agent always has its rules) at the cost of a round-trip
  and losing the rules from the main thread. The cluster boundaries held across all six passes
  with only one node splitting across two clusters (rule 19), so the precondition is now met —
  this is the next structural question, if there is one.
