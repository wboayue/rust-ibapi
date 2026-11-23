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

### Historical schedule honors useRTH

*Live 2026-11-03 off-hours, paper, server 226; #1147.*

- `reqHistoricalData` with `whatToShow=SCHEDULE`, 5 days, `useRTH=1` vs `useRTH=0`:
  - AAPL (STK): RTH 09:30–16:00 ET; extended 04:00–20:00 ET. Five sessions each.
  - ES (FUT, CME, next quarter): RTH 08:30–16:00 CT; extended 17:00 (prior day) – 16:00 CT.
- Extended futures sessions are keyed by trade date: the session dated 11-02 starts Sunday
  11-01 17:00. The window also shifts forward one trade date (the session underway counts),
  so `reference` differs from the start date and the RTH and extended lists are not
  index-aligned.
- Settles that the flag changes the result; the C# client passes it through unchanged.
- **Not probed:** holidays / early closes, non-US exchanges, `TradingHours` on other
  security types (options, FX, crypto).

### Historical bars for a continuous future take no end date

*Live 2026-11-16 Mon 02:20 ET (CME Globex open), paper, server 223.*

- `reqHistoricalData` on ES `CONTFUT`, 1-hour TRADES bars: a 2-day duration with no end date
  returns 16 bars (2026-11-12 14:30 to 2026-11-13 21:00 UTC). The same request with an end date
  of 2026-11-13 21:00 UTC returns error 10339, "Setting end date/time for continuous future
  security type is not allowed". `between` sent an end date, so it got 10339 too.
- Settles that `between` on a continuous future must omit the end date and ask for a duration
  reaching back to `start` from now.
- **Not probed:** `keepUpToDate` streaming; CONTFUT on another exchange.

### Continuous future: schedule rejects an end date, ticks accept one; bar depth and cost

*Live 2026-11-22 Sun 19:56 ET (CME Globex open), paper, server 226. ES `CONTFUT`, extended
hours, TRADES.*

- `reqHistoricalData` with `whatToShow` SCHEDULE, 5 D: with an end date of 2026-11-13 20:00 UTC,
  error 10339, the same text as for bars. With no end date, 5 sessions.
- `reqHistoricalTicks`, 100 trade ticks: with an end date of 2026-11-13 20:00 UTC, 114 ticks
  ending 19:59:52; with a start date of 15:00 UTC, 128 ticks from 15:00:00. No 10339 either way.
- Bars with no end date: the bar-size step limit still applies. 1-second bars over 1800 S
  return 1800 bars; over 1 D they draw 162 "invalid step: 1".
- Longer durations are accepted but slow. 1 M of 30-second bars took 74 s (55433 bars), 1 M of
  1-minute bars 73 s (27720), 1 Y of hourly bars 41 s (5896), 5 Y of hourly bars 122 s
  (16063). 2 M of 30-second bars and 1 Y of 1-minute bars did not finish within 90 s.
- History is shallower than asked: 5 Y of hourly bars starts 2023-09-27, and 20 Y of daily bars
  returns 1391 bars from 2021-06-07.
- Settles that schedule's `ending()` needs the same up-front rejection as bars and ticks'
  does not, and that `between` on a continuous future pays for `now - start` in wall-clock
  time rather than drawing an error.
- **Not probed:** whether the 90 s timeouts end in data or an error; bid/ask and midpoint
  ticks; daily bars over 5 Y or less.
