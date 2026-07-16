# PR-D — Fundamentals deprecation/removal decision (TWS 10.47)

Part of the TWS 10.47.01 C# reference sync. **Gates PR-B** (proto regen).

## Background
IBKR fully removed the fundamental-data feature in TWS 10.47 (commit `d8c3743c`):
`reqFundamentalData`, `cancelFundamentalData`, both callbacks, tick `FUNDAMENTAL_RATIOS = 47`, and 3
proto files (`FundamentalsData`, `FundamentalsDataRequest`, `CancelFundamentalsData`). **No
replacement** named; `fundamentals.html` doc now redirects to Wall Street Horizon. Confirmed via
2026 production release notes.

Our `src/fundamental/` module (public client methods, `common/encoders.rs`, `common/decoders.rs`,
`testdata/builders/fundamental.rs`) depends on those now-deleted proto structs. `cargo run -p
proto-gen` sparse-clones upstream, so a plain regen deletes them and breaks the build.

## Option 1 — Vendor + deprecate (recommended)
Keep the feature working; signal upstream direction.
- Retain the 3 proto messages locally. Two mechanisms — pick one:
  - (a) Extend `tools/proto-gen` to overlay a repo-local `proto/vendored/` dir onto the sparse
    clone before compiling (preferred — keeps generation reproducible).
  - (b) Hand-maintain the 3 structs appended to `src/proto/protobuf.rs` with a `// vendored:
    removed upstream in 10.47` banner (simpler, but drifts from the generated file).
- Add `#[deprecated(note = "IBKR removed fundamental data from the TWS API in 10.47; endpoint may
  stop responding")]` on the public fundamental methods.
- Note in `docs/migration-3.0.md` + README.
- Revisit removal once a live gateway confirms the server endpoint 404s.

## Option 2 — Remove (match upstream, v3.0 break)
- Delete `src/fundamental/` (impl, sync/async, tests), the public client methods, and
  `src/testdata/builders/fundamental.rs`.
- Drop `FUNDAMENTAL_RATIOS` from `TickType` (keep the enum arm exhaustive per the TickEFP precedent
  if an incoming id must still map to a known-but-unclaimed variant).
- Then PR-B's regen is clean (the 3 structs vanish with no dangling refs).
- `CHANGELOG.md` `Removed` + `docs/migration-3.0.md` entry (breaking).

## Decision
_Pending user._ Recommendation: Option 1. Whichever is chosen, this PR (or its proto-retention
mechanism) must land before PR-B so the regenerated `protobuf.rs` compiles.
