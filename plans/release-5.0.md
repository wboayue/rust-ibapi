# 5.0 release backlog

Breaking changes held back from 4.x minors. Land each one in the 5.0 cycle, and
give it a section in `docs/migration-5.0.md`.

## Done

- utoipa 5 → 6 (#870): bumped, hand-written `OrderStatusKind`/`TimeInForce`
  schemas verified still `{"type":"string"}`, migration-5.0.md §1.

## Release prep

- `docs/migration-4.0.md` §16–§28 are listed as "Unreleased" but ship in 5.0,
  not a 4.x minor. Before tagging, move them into `migration-5.0.md` (renumber,
  fix CHANGELOG `§N` pointers) or relabel them in 4.0's release table.
