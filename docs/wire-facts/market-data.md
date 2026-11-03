# Market data wire facts

Generic ticks, option computations and tick types. Format: [README](README.md).

### ETF NAV generic ticks

*Live 2026-10-04 overnight, paper Gateway, server 220, SPY/QQQ; #946, #952. RTH re-run
2026-10-27, server 226.*

- `576` and `578` → 321 "Incorrect generic tick list". TWS's legal list for STK includes
  `577` (EtfNavLast), `614` (EtfNavMisc high/low), `623` (EtfFrozenNavLast), not 576/578.
- `577` → tick 96 `EtfNavLast` in RTH (SPY 769.72 vs last 769.75; QQQ 17 ticks in 45s, SPY
  1–4 per 20–60s). Nothing outside RTH.
- `614` → accepted, but no tick 98/99 in 60s SPY / 45s QQQ during RTH, no error.
- `623` → nothing in RTH (expected: frozen value, after close).
- **Not probed:** 623 → tick 97 after close with Frozen; 614 over a longer window (#946).

### Option computation volatility is a fraction

*Live 2026-10-27 RTH, paper Gateway, server 226, SPY 20261030 C770 @768.*

- `calculate_option_price`: 0.4 → $10.72, 0.1 → $2.12, 40.0 → $720 (4000% vol). So 0.4 means
  40%. Rustdoc previously said "percentage (e.g. 20.0)"; corrected.
- Same concept, different unit from VOL orders: [orders](orders.md#vol-order-volatility-is-percent).

### Option snapshot computations

*Live 2026-10-29 RTH, paper Gateway, server 226, AAPL 20261113 350C; #1121.*

- A snapshot with no generic ticks delivers all four computations: Bid, Ask, Last, Model.
  Model's `present_value_dividend` is None.
- STK snapshots fill every `Quote` field (12 ticks; bid/ask/last arrive as PriceSize), RTH and
  the overnight session alike (2026-10-29 23:55 ET: AAPL/SPY/IBKR, real bid/ask). No -1 seen.
- **Not probed:** a weekend (true no-quote) snapshot.

### TickEFP is never sent

*Live 2026-05-27, paper Gateway, server 220+; removed in v3.0 (#651).*

- AAPL, SPY, IBKR, SAP@IBIS, SIE@IBIS for ~5s each: 251 frames, all proto-framed, zero msg id
  47. The Eurex single-stock futures underlyings, the likeliest EFP triggers, returned other
  ticks normally.
- C# has no `TickEFPEventProtoBuf` handler, no `TickEFP.proto`, no version constant: EFP is
  text-only by design. Reintroduce only if those appear upstream.
