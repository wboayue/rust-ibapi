# 5.0 release backlog

Breaking changes held back from 4.x minors. Land each one in the 5.0 cycle, and
give it a section in `docs/migration-5.0.md`.

## Done

- utoipa 5 → 6 (#870): bumped, hand-written `OrderStatusKind`/`TimeInForce`
  schemas verified still `{"type":"string"}`, migration-5.0.md §14; feature renamed `utoipa-6`.
- migration-4.0.md §16–§28 + three behavioral bullets moved to
  migration-5.0.md §1–§13; CHANGELOG pointers updated.
