# 5.0 release backlog

Breaking changes held back from 4.x minors. Land each one in the 5.0 cycle, and
give it a section in `docs/migration-5.0.md`. Shipped items are recorded there
and in `CHANGELOG.md`.

## Open

None queued.

## Notes

- Adding utoipa 7: `utoipa-7 = ["utoipa", "dep:utoipa7"]` + a newest-wins
  `cfg` on the `extern crate` aliases in `lib.rs` (pattern from #884).
