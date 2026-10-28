---
id: user-docs-sync
title: README and the migration guide ship with the change that breaks them
cluster: docs
status: active
triggers:
  - removing or renaming anything public
  - changing a public field's type or a return type's shape
  - adding a builder method or feature flag a user upgrading would search for
  - about to grep for a name you just renamed
symbols: [README.md, migration-5.0.md]
related: [changelog-entry, public-api-examples, restrict-after-callers, modernize-touched-modules]
precedents: ["#549", "#771", "#884"]
memory: [feedback_md_doc_snippets_rot_silently, feedback_field_removal_breaks_public_contract]
---

Treat `README.md` and the **next major's** migration guide as public API: a breaking change
updates both in the same PR. 5.0 shipped (2026-10-04), so `docs/migration-5.0.md` is a frozen
record like `docs/migration-4.0.md`, `docs/migration-3.0.md`, and `MIGRATION.md`; a breaking
change on 5.x means starting 6.0 and `docs/migration-6.0.md` — ask before doing so. Frozen
guides get no new sections, but a rename can still strand their prose, so they stay in the grep
set below.

Update the migration guide when the PR:

- removes or renames a public type, struct field, enum variant, method, or re-export;
- changes a public field's type (`String` → typed enum, `bool` → mode enum);
- changes the shape of a return type (a `Subscription<T>::next()` envelope, a new `Result`
  variant);
- adds or removes a public builder method, callback hook, or feature flag an upgrading user
  would search for.

Update `README.md` when the PR touches code in a README example, removes a variant its `match`
blocks handle, or establishes a new happy-path idiom (`is_terminal()` over magic-string
compares). Cross-link both ways: a new migration section is reachable from README prose near
the example it explains, and the README's "📚 Migrating?" list gains the new guide.

## The grep is not enough — read the hits

Before opening the PR, grep `README.md`, every `docs/*.md`, and module-level rustdoc for every
name you changed, removed, or replaced. Stale references are blockers.

Then **read each hit**. `cargo test --doc` compiles only `# Examples` blocks in `.rs` files; a
```rust block in `README.md` or `docs/*.md` is prose with no CI gate. Mentally compile each
snippet: do the identifiers exist, do the methods chain on those receivers, are the field types
still spelled that way?

When a *field* disappears, widen past rustdoc: a public method deriving its return value from
the field breaks its contract with no doc mentioning the field — `examples/` and `docs/` show it.

## Precedents

- #549 — found six `order_builder::market_order(...).condition(...).build()` chains in
  `docs/api-patterns.md` (`Order` has no `.condition()`) and `Order { lmt_price: ..., tif:
  "GTC".to_string() }` blocks in `docs/order-types.md` (real fields: `limit_price`,
  `tif: TimeInForce`), wrong for months because nothing compiles them.
- #771 — 4.0 prep: three unreleased sections had accumulated in the shipped 3.0 guide and three
  breaking changes had changelog entries but no guide section — per-PR sync had targeted the
  wrong file since 3.3.0.
- #884 — 5.0: the same failure again; thirteen "Unreleased" sections in the shipped 4.0 guide
  moved to `docs/migration-5.0.md` as §1–13, and the #845 changelog entry had been filed under
  4.2.0 though it merged after the tag.
