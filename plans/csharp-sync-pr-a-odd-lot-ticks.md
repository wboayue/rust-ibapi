# PR-A — Odd-lot bid/ask tick types (server 225 / v10.46)

Part of the TWS 10.47.01 C# reference sync. Highest user value; fully self-contained
(no proto regen, no fundamentals dependency).

## Background
IBKR added odd-lot bid/ask market-data ticks in v10.46 (server version 225). They ride the
existing `TickPrice`/`TickSize` messages via new `TickType` field ids. Our `TickType` enum
(`src/contracts/tick_types/mod.rs`) tops out at 104 (`DelayedYieldAsk`), so ids 105–110 currently
decode to `TickType::Unknown` — the data is silently dropped.

## Changes
Add variants to `TickType` (`src/contracts/tick_types/mod.rs`):
- `OddLotBid = 105`
- `OddLotAsk = 106`
- `OddLotBidSize = 107`
- `OddLotAskSize = 108`
- `OddLotBidExch = 109`
- `OddLotAskExch = 110`

Wire through all three conversions (currently ending near lines 226 / 340 / 448):
- `impl From<i32> for TickType` — add the 6 id arms before the `_ => Self::Unknown` catch-all.
- `impl From<&str> for TickType` — add the wire-string arms (verify exact tokens against C#
  `TickType.cs`; e.g. `"oddLotBid"` etc.).
- `impl Display for TickType` — round-trip strings.

## Verification
- Grep C# `source/csharpclient/client/TickType.cs` for the exact `field`/`FieldToString` tokens
  for ids 105–110 — do not guess the `From<&str>`/`Display` strings.
- Unit tests: each new variant round-trips `From<i32>` → `Display` → `From<&str>`; derive expected
  ids from the variant (rule 21 — no bare hardcoded 105).
- `CHANGELOG.md` `Added`: odd-lot bid/ask tick types (#PR).

## Checks
`cargo fmt`; all 3 clippy configs; all 3 rustdoc configs; `just test`; sync + async + all-features.
