# PR-C — 4-minute historical bar size (v10.44)

Part of the TWS 10.47.01 C# reference sync. Trivial, self-contained.

## Background
Upstream added `"4 mins"` bar support in v10.44 (Sep 2025). Our `BarSize` enum
(`src/market_data/historical/mod.rs`) has `Min2` (line ~201), `Min3` (~203), `Min5` (~205) but no
`Min4`.

## Changes
- Add `Min4` variant between `Min3` and `Min5` with doc `/// Four-minute bars.`
- Wire its wire-string `"4 mins"` in the `BarSize` Display / to-wire mapping (match the existing
  `Min3 => "3 mins"` idiom — verify the exact token against C# / captured wire).
- Verify real-time bars are unaffected (`src/market_data/realtime/mod.rs` has a separate, smaller
  bar-size set; 4-min is historical-only unless C# says otherwise).

## Verification
- Unit test: `Min4` maps to `"4 mins"` and back if `FromStr` exists.
- `CHANGELOG.md` `Added`: 4-minute historical bar size (#PR).

## Checks
`cargo fmt`; all 3 clippy configs; all 3 rustdoc configs; `just test`; sync + async + all-features.
