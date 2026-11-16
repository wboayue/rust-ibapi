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

### Bar volume and WAP on non-TRADES bars

*Live 2026-11-15 Sun 22:25 ET (FX and CME Globex open, US equities overnight), paper, server
226; raw capture.*

- `reqRealTimeBars` MIDPOINT, EUR.USD and ES: `RealTimeBarTick.volume` (7) and `WAP` (8)
  **absent** in 7 of 7 frames; `count` sent as `-1`. TRADES on ES: all three sent with real
  values. EUR.USD TRADES draws 420 (no Last data for CASH).
- `reqHistoricalData` MIDPOINT and BID, 1-hour bars, AAPL, EUR.USD and ES: `volume` and `WAP`
  **sent** as `"-1"`, `barCount` as `-1`. TRADES sends real values.
- Settles that the two bar types differ: realtime leaves the fields out, historical sends a
  `-1` sentinel. C# reads an absent realtime value as unset (`decimal.MaxValue`).
- **Not probed:** ASK bars; AAPL realtime bars (no data overnight); `HistoricalDataUpdate`
  (keep-up-to-date) bars.

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
