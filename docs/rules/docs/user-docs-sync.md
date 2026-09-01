---
id: user-docs-sync
title: README and the migration guide ship with the change that breaks them
cluster: docs
status: active
triggers:
  - removing or renaming anything public
  - changing a public field's type or a return type's shape
  - adding a builder method or feature flag a 2.x user would search for
  - about to grep for a name you just renamed
symbols: [README.md, migration-4.0.md]
related: [changelog-entry, public-api-examples, restrict-after-callers, modernize-touched-modules]
precedents: ["#549", "#771"]
memory: [feedback_md_doc_snippets_rot_silently, feedback_field_removal_breaks_public_contract]
---

Treat `README.md` and the current major's migration guide — `docs/migration-4.0.md` — as part
of the public API. A breaking change updates both in the same PR — a migration guide that tells
users to adopt patterns which no longer compile is worse than no guide. Earlier guides
(`docs/migration-3.0.md`, `MIGRATION.md`) are frozen records of shipped transitions: new
sections never go there, but a rename can still strand their prose, so they stay in the grep
set below.

Update `docs/migration-4.0.md` when the PR:

- removes or renames a public type, struct field, enum variant, method, or re-export;
- changes a public field's type (`String` → typed enum, `bool` → mode enum);
- changes the shape of a return type (a `Subscription<T>::next()` envelope, a new `Result`
  variant);
- adds or removes a public builder method, callback hook, or feature flag that a 2.x user
  would find by searching.

Update `README.md` when the PR touches code shown in a README example, removes a variant its
`match` blocks handle, or establishes an idiom that should now be the happy path (`is_terminal()`
over magic-string compares).

Cross-link both ways: a new migration section should be reachable from the README prose near
the example it explains, and the README's "📚 Migrating?" pointer must keep resolving.

## The grep is not enough — read the hits

Before opening the PR, grep `README.md`, every `docs/*.md`, and module-level rustdoc for every
name you changed, removed, or replaced. Stale references are blockers, not nits.

Then **read each remaining hit**. `cargo test --doc` compiles `# Examples` blocks in `.rs`
files and nothing else; a ```rust block in `README.md` or `docs/*.md` is prose. There is no CI
gate. Mentally compile each snippet: do those identifiers still exist, do those methods chain
on those receivers, are the field types still spelled that way?

Widen the grep past rustdoc when a *field* disappears. A public method that derives its return
value from the removed field breaks its observable contract even though no doc mentions the
field — `examples/` and `docs/` are where that surfaces.

## Precedents

- #549 — the order-construction sweep found six `order_builder::market_order(...).condition(...)
  .build()` chains in `docs/api-patterns.md` (`Order` has no `.condition()`; the fluent
  `OrderBuilder` does) and `Order { lmt_price: ..., tif: "GTC".to_string() }` blocks in
  `docs/order-types.md` (the real fields are `limit_price` and `tif: TimeInForce`). Both had
  been wrong for months because nothing compiles them.
- #771 — 4.0 release prep: the live guide moved to `docs/migration-4.0.md`. Three unreleased
  sections had accumulated in the 3.0 guide (§35–37) and three breaking changes had changelog
  entries but no guide section at all (`Liquidity::Unknown`, `Notice.request_id`,
  `DATA_ADVISORY_CODES`) — per-PR sync had been targeting the wrong file since 3.3.0 shipped.
