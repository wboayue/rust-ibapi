# Market data wire facts

Generic ticks, option computations, tick types and historical bars. Format: [README](README.md).

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

### CFD reroute ends the request on the TWS side

*Live 2026-11-10 RTH, paper Gateway, server 226, SMART CFDs AAPL/BMW/IBUS500/IBDE40; #1197.*

- Stock CFDs (AAPL con 120549942, BMW con 102284037): `reqMktData` and `reqMktDepth` (5
  rows) each draw exactly one frame, `RerouteMktDataReq` / `RerouteMktDepthReq`, naming the
  underlying's stock contract (AAPL 265598, BMW 14094) and exchange `SMART`. After it
  comes nothing: no ticks, no error, for 15s.
- TWS has already ended the CFD request. `cancelMktData` on the rerouted id draws 300
  "Can't find EId with tickerId". `cancelMktDepth` draws nothing, so the depth side was
  checked by slots instead: with six rerouted depth requests left uncancelled, a SMART-depth
  stock request is still accepted. The control, real depth requests left uncancelled,
  draws 309 "Max number (3) of market depth requests" on the third.
- Index CFDs (IBUS500, IBDE40) are not rerouted: `reqMktData` draws 354 (not subscribed)
  and `reqMktDepth` draws 10092 (deep data not supported for the security type/exchange).
- **Not probed:** delayed market data (type 3/4) on a CFD; snapshot requests; FX or
  commodity CFDs; a window longer than 15s; the async client.

### Bar trade fields (volume, WAP, count)

*Live 2026-11-15 Sun 22:25 ET (FX and CME Globex open, US equities overnight), paper, server
226; raw capture.*

- `reqRealTimeBars` MIDPOINT, EUR.USD and ES: `RealTimeBarTick.volume` (7) and `WAP` (8)
  **absent** in 7 of 7 frames; `count` sent as `-1`. TRADES on ES: all three sent with real
  values. EUR.USD TRADES draws 420 (no Last data for CASH).
- `reqHistoricalData` MIDPOINT and BID, 1-hour bars, AAPL, EUR.USD and ES: `volume` and `WAP`
  **sent** as `"-1"`, `barCount` as `-1`. TRADES sends real values.
- Settles that the two bar types differ: realtime leaves the fields out, historical sends a
  `-1` sentinel. C# reads an absent realtime value as unset (`decimal.MaxValue`).

*Live 2026-11-16, paper, server 226; raw capture; #1247.*

- A TRADES bar in which nothing traded sends volume `"0"`, count `0` and WAP as a price, not
  the `-1` sentinel: 428 of 541 historical 5-second AAPL premarket bars, 6 of 597 ES, 3 of 3
  realtime BTC/PAXOS.
- Realtime BID and ASK (EUR.USD, BTC/PAXOS) match MIDPOINT: volume and WAP absent, count `-1`.
  Historical ASK and BID_ASK (ES), MIDPOINT and ASK (EUR.USD), MIDPOINT (BTC/PAXOS): `-1` for
  all three. Keep-up-to-date `HistoricalDataUpdate` bars for MIDPOINT and BID: `-1` for all
  three (16 of 16, EUR.USD and BTC/PAXOS).
- ADJUSTED_LAST and AGGTRADES carry real trade values. HISTORICAL_VOLATILITY and
  OPTION_IMPLIED_VOLATILITY (AAPL) send volume `"1"`, count `0` and a WAP.
- Keep-up-to-date TRADES on BTC/PAXOS, 8 minutes: 97 update bars, all with `bar_count` (81
  with trades, 16 quiet). TWS answered 10299 ("use AGGTRADES") and streamed anyway.
- Settles that `-1` and absence both mean no trade data, while a quiet TRADES bar is a real
  zero.
- **Not probed:** AAPL realtime bars (no data overnight); a keep-up-to-date TRADES update bar
  without `bar_count` (#1057), for which stocks and futures need up-to-the-second data (2188 on
  the probing account).

### One-second bars reach back at most 2000 S

*Live 2026-11-23 21:21-21:32Z (after the US close; the ES runs during the 21:15-21:30Z CME halt
and after it), paper TWS, server 223. TRADES, regular hours.*

- AAPL, ending 2026-11-23 21:00 UTC: 1-second bars over 1800 S return 1800 bars and over 2000 S
  2000 bars (20:26:40 to 20:59:59 UTC). 2001, 2100, 2200, 2300, 2399, 2400 and 3600 S each draw
  162 "Historical Market Data Service error message:invalid step: 1", about 0.2 s after the
  request.
- 5-second bars: 3600 S returns 720 bars (0.4 s), 14400 S 2880 (0.6 s), 28800 S 5760 (11.4 s).
  Durations count trading seconds: the 28800 S request starts at 2026-11-20 19:30 UTC, the
  Friday session, while its reported window start (08:00 ET on the 23rd) is wall clock.
- ESZ6 (CME, no end date), halted and open alike: 1-second bars over 1800 and 2000 S succeed,
  2400 and 3600 S draw the same 162; 5-second bars succeed at 3600, 14400 and 28800 S. Bar
  counts fall short of the duration (1192 bars over 1800 S with CME open); the probe did not
  establish why. 28800 S took 54 s on both runs, and its reported window ends about 11 s
  after the previous request finished, so it was sent late or re-sent; not diagnosed.
- Settles the boundary: 2000 S passes and 2001 S fails for 1-second bars (bisected on AAPL;
  ESZ6 agrees at 2000 and 2400 S). Inferred: the limit is on the step count (duration over
  bar size), not on bars returned. ESZ6 returns fewer bars than steps (1192 over 1800 S), so
  its 2400 S request would hold about 1600 bars, yet it failed. Nor is it a bar cap shared by
  every bar size: 5760 five-second bars passed.
- **Not probed:** 5-second bars above 28800 S; other bar sizes' limits; ES in the
  22:00-23:00Z closure; the Gateway.

### Weekly and monthly bars are dated at the end of the period

*Live 2026-11-23 21:20Z (Monday, after the US close), paper TWS, server 223; AAPL TRADES, 3 M
ending now, regular hours; wire dates read from a recording.*

- Each bar carries a plain `YYYYMMDD` (decoded as `BarTimestamp::Date`).
- Weekly (14 bars): `20260828`, `20260904`, ... `20261120`, every complete week dated its
  Friday, then `20261123` for the current week.
- Monthly (4 bars): `20260831` (Mon), `20260930` (Wed), `20261030` (Fri; 31 Oct 2026 is a
  Saturday), then `20261123` for the current month.
- The current, incomplete week and month are dated today, the last trading day so far. The
  first weekly bar is cut by the window start (Sun 23 Aug) and still dated its end, 28 Aug.
- Intraday bars are stamped with their start (1-second bars over 20:30:00-21:00:00 UTC run
  from 20:30:00 to 20:59:59; [above](#one-second-bars-reach-back-at-most-2000-s)). Weekly and
  monthly bars are stamped with the end of the period instead; `HistoricalData::trim_to`
  documents both rules (#1267).
- **Not probed:** daily bars in this run; a holiday on a period's last weekday, so the last
  trading day is not separated from the last weekday; other instruments; the Gateway.

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
  return 1800 bars; over 1 D they draw 162 "invalid step: 1". The boundary is 2000 S
  ([one-second bars](#one-second-bars-reach-back-at-most-2000-s)).
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

### Continuous future 1-second bars: the 2000 S limit counts from now

*Live 2026-11-23 21:22Z (CME halt) and 21:31Z (CME open), paper TWS, server 223; ES `CONTFUT`,
1-second TRADES, `between`.*

- `between` sends no end date for a continuous future and asks for `now - start`, rounded up
  to a whole second ([no end date](#historical-bars-for-a-continuous-future-take-no-end-date)).
- `[now - 3000 s, now - 2500 s)` draws 162 "invalid step: 1" on both runs, although the range
  is 500 s.
- `[now - 1500 s, now - 1000 s)` returns exactly 500 bars on both runs (21:06:07 to 21:14:26
  UTC with CME open).
- Settles that the [2000 S limit](#one-second-bars-reach-back-at-most-2000-s) applies to
  `now - start`: a 1-second `between` on a continuous future must start no more than about
  2000 s before now, however short the range.
- **Not probed:** ranges starting between 2000 and 2500 s back; other bar sizes.

### Size fields are always sent

*Live 2026-11-15 Sun 22:25 ET, paper, server 226; raw capture, same session as above.*

- Every frame carried its size: `TickSize.size` (35 frames, tick types 0/3/5/8; `"0"` sent
  explicitly twice), tick-by-tick Last (ES, 9) and BidAsk (EUR.USD and ES, 40) sizes, and
  `MarketDepth` size (EUR.USD and ES, 80, insert and update). `TickReqParams.minTick` and
  `snapshotPermissions` were sent for all three contracts (`0` explicitly for EUR.USD).
- So decoding an absent size as `0` changes nothing observed; these stay `f64`.
- **Not probed:** `MarketDepthL2` (AAPL depth drew 10092); RTH equity ticks; AAPL
  tick-by-tick (no data overnight).

### `TickReqParams` precision fields

*Live 2026-11-15 Sun 22:25 ET, paper, server 226; raw capture; #1251.*

- `reqMktData` first frame: `lastPricePrecision` and `lastSizePrecision` sent as `"0.000001"`
  for AAPL, absent for EUR.USD and ES. Steps, not digit counts; the same values
  `ContractData` carries ([contracts](contracts.md#event-contract-minalgosize-and-precision-fields)).
- Not decoded: one stock out of three contracts, with no documented meaning.
- **Not probed:** options, crypto, non-US stocks.

### Index ticks

*Live 2026-11-24 Tue 09:41-09:44 UTC (Xetra open; CBOE, OSE and HKFE closed), paper, server 223;
#1284. US trading hours: live 2026-11-24 16:00-16:02 UTC (CBOE and Nasdaq open), paper TWS,
server 223.*

- `reqMktData` real-time on SPX (CBOE), DAX and ESTX50 (EUREX), N225 (OSE.JPN) and HSI (HKFE):
  354 for each; this account has no index subscription. Delayed data (type 3) arrives after a
  10167 notice.
- Delayed, on SPX, DAX, N225 and HSI: one `DelayedBid` and one `DelayedAsk` of `-1` with size
  `0` at the start, never a real one. `DelayedLast` is a real price with size `1`, closed markets included (SPX
  `7236.85`, N225 and HSI their last value), with a `DelayedLastTimestamp`. No `-1` last price
  arrived.
- `DelayedOpen` is `0` (a price tick, no size) on all four, DAX included while it traded.
  `DelayedHigh` and `DelayedLow` are real; SPX sent neither while CBOE was closed.
- DAX during its session: a new `DelayedLast` every 5 seconds, size `1` each time.
- Settles that an index's bid and ask are the no-quote placeholder of every other contract
  (`-1` with size `0`, #1284), and that its last price is a value, not a placeholder, even with
  the market closed. Inferred: the size `1` is not a traded size.
- **US trading hours**: delayed SPX (CBOE), NDX (NASDAQ) and VIX (CBOE) the same way. One `DelayedBid`
  and one `DelayedAsk` of `-1` with size `0` first, then a `DelayedLast` with size `1` and a
  `DelayedLastTimestamp` every 5 seconds (SPX `7158.43`, `7154.96`, ...). `DelayedOpen` was `0`
  on all three with the session open; `DelayedHigh`, `DelayedLow` and `DelayedClose` (the
  previous close) were real. `DelayedVolume` came as a small count that grew (SPX `4535`, then
  `4565` 10 s later; NDX `4561`; VIX `1776`, then `1778`); what it counts was not
  established, since an index does not trade. Real-time on SPX and VIX: 354.
- **Not probed:** real-time index data (no subscription); delayed ESTX50; an index before its
  first value of a day; snapshots.

### Delayed volume is negative or wrong on the wire

*Live 2026-11-24 01:00Z (Monday 20:00 ET, after hours), paper TWS, server 223; raw capture.*

- `reqMktData` for US stocks under market data type 3 (delayed, streaming, generic tick 165)
  and 4 (delayed frozen, snapshot). `TickSize` tick type 74 (`DelayedVolume`) carried
  negative decimal strings for MSFT (`"-235087060"`, then `"-234537060"` 10 s later), NVDA,
  INTC, F, SPY and QQQ, in both modes. The raw frame holds the text: field 3, length 10,
  `2d 32 33 35 30 38 37 30 36 30` (`-235087060`). The decoder parses that string to the same
  `f64`; nothing narrows it to 32 bits. An earlier run (snapshot, 23:49Z) saw MSFT
  `-1232329840`.
- The values also disagree with the day's volume where they are positive. Daily TRADES bars
  for 2026-11-23, extended hours: AAPL 41,406,011 against `DelayedVolume` 48,288,217; AMD
  42,241,724 against 2,119,449,190; IBKR 4,649,304 against 1,226,539,095. Neither the raw
  value nor a 32-bit unwrap of it (adding 2^32, with or without a factor of 100) matches the
  bar volumes, so a caller cannot repair the value or tell a bad positive one from a real one.
- Other size ticks in the same runs were plausible and never negative: `AvgVolume` (21, e.g.
  MSFT `"596315"`), `DelayedBidSize` (69) and `DelayedAskSize` (70).
- TSLA drew 10089 and no ticks; SPY and QQQ answered with delayed data.
- **Not probed:** real-time `Volume` (8) (no real-time US equity permission on this account);
  delayed volume during regular hours; other markets.

### Market depth rates, rows and refusals

*Live 2026-11-24 Tue 09:16-09:59 UTC (FX and Xetra open; US equities overnight), paper, server
223; nautechsystems/nautilus_trader#5395.*

- `reqMktDepth` EUR.USD (IDEALPRO), 10 rows, two concurrent 26-minute subscriptions, one with
  `isSmartDepth` false and one true: both sides updated every minute, 2,400 to 9,200 updates per
  side per minute; the longest silence was 5.4 s, on both sides at the same moment. Totals per
  run: about 218,000 updates, 8,900 inserts and 9,000 deletes.
- `isSmartDepth` false sends `MarketDepth` rows; true sends `MarketDepthL2` rows with market
  maker `IDEALPRO` and `isSmartDepth` true, after a 2152 notice "Exchanges - Depth: IDEALPRO;".
- Rows: 50 requested on EUR.USD gives positions 0-9 (10 rows); 5 requested gives 0-4, built with
  10 inserts and then updates only.
- A fourth concurrent request, from any client id on the same TWS, draws 309 "Max number (3) of
  market depth requests has been reached" and ends the stream.
- Refusals: AAPL `SMART` without smart depth draws 10092 and ends; AAPL `ISLAND` draws 10089
  as a notice and then nothing (the stream stays open); SAP `IBIS` and ES draw 354. AAPL smart
  depth draws a 2152 listing the exchanges without depth permission, then nothing overnight.
- No 317 (depth reset) or 316 (halted) arrived in 52 subscription-minutes.
- No one-sided stall (one side silent while the other updates) reproduced on FX depth.
- **Not probed:** US stock depth during US trading hours; a 317 reset (which rows TWS resends
  after it); depth longer than 26 minutes.

### Tick-by-tick sizes and `IgnoreSize`

*Live 2026-11-16 03:33Z (Sunday evening ET), paper Gateway, server 226, BTC/PAXOS; #1244.*

- `last` / `all_last` (24 frames) and `bid_ask` (121 frames, both `IgnoreSize` values): every
  frame carried its size fields (`size`; `size_bid` and `size_ask`).
- `mid_point` (84 frames): no size field in any frame.
- `bid_ask` with `IgnoreSize::Yes` (one window, 40 updates read): every update changed a price
  (0 of 39 consecutive pairs with both prices unchanged). With `IgnoreSize::No` (two windows, 40
  updates read in each), 12 of 39 and 4 of 39 pairs changed only sizes. Consistent with `Yes`
  dropping size-only updates rather than the size fields.
- Requested right after the `IgnoreSize::No` subscription on the same contract was cancelled,
  `IgnoreSize::Yes` got no frames and no error in 25 s. Requested first, it delivered. One run,
  not explained.
- Historical ticks (live 2026-11-15 08:27 ET, Sunday, paper Gateway, server 226; AAPL ticks of
  2026-11-13): every trade tick (1018) carried `size` and every bid/ask tick (17) `size_bid` and
  `size_ask`; every midpoint tick (19) carried `size`, always `"0"`.
- Historical bid/ask ticks with `IgnoreSize` (live 2026-11-16 05:02Z, Monday 00:02 ET, paper
  Gateway, server 226; AAPL, 200 ticks requested ending 2026-11-13 21:00 UTC, all returned
  ticks fall in 20:59:58-59 UTC; each request drew notice 2188 and still delivered): decoded
  from the capture, `Yes` 475 ticks and `No` 1009, every one with `size_bid` and `size_ask`. 0
  of 474 consecutive `Yes` pairs and 534 of 1008 `No` pairs had both prices unchanged: `Yes`
  drops size-only updates here too.
- ES and EUR.USD tick-by-tick sizes, all sent: [Size fields are always sent](#size-fields-are-always-sent).
- **Not probed:** live stock ticks during RTH, live accounts, TWS rather than the Gateway.
