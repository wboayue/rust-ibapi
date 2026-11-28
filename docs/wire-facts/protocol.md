# Protocol wire facts

Connection, request ids, cancels and end markers. Format: [README](README.md).

### Duplicate client id (326)

*Live 2026-10-26, paper Gateway, server 226; #1091.*

- Connecting with a client id already in use: after `start_api`, TWS sends exactly one frame, a
  protobuf Error (msg id 4, request id -1, code 326, "Unable to connect as the client id is
  already in use. Retry with a unique client id."). No NextValidId or ManagedAccounts. Socket
  closes ~0.7s later.
- A clean drop of the holder frees the id immediately.
- Half-open socket (simulated with a TCP proxy that closed the client side and held the TWS
  side 10s): each reconnect attempt got 326 until TWS closed its side, then the next attempt
  reconnected. So 326 is transient on reconnect (retry) and fatal on the initial connect.

### Request id range

*Live 2026-09-11, paper Gateway, server 220; #789, #912 (`REQUEST_ID_FLOOR` /
`REQUEST_ID_CEILING` in `src/client/ids.rs`).*

- Data frames (ContractData / End) echo every id tried: 0, 9000, 1<<24, 1<<30, i32::MAX-1,
  i32::MAX, -2, -1000, i32::MIN+1, i32::MIN.
- Error frames (code 200) echo all of those **except i32::MAX and i32::MIN**: field 1 arrives
  absent, decodes as -1, and the error cannot be routed. An allocator must stop at i32::MAX-1.
- `-1` is IB's NO_VALID_ID sentinel; the C# client does no sign validation.

### Cancel after a request-bound terminal error

*Live 2026-11-24 off-hours, paper TWS, server 223, sync and async; first sweep 2026-10-26, paper
Gateway; #1094, #1097, #1096.*

TWS reply to a cancel sent after the request already failed. Observed while the type still sent
the cancel; types whose code is verified here now skip it, so the reply no longer occurs for them.

| Request | Terminal error | Cancel reply |
| --- | --- | --- |
| market_data (stream and snapshot) | 200, 354, 10168 | 300 |
| realtime_bars | 200, 354, 420 | 300 |
| tick_by_tick | 200, 10189 | 300 |
| market_depth | 200, 354 | 310 |
| market_depth | 10092 (no depth for the security type and exchange) | none |
| historical_data (subscribe / fetch) | 200, 162, 10314 | 366 "No historical data query found" |
| historical_data (subscribe_with_updates) | 162, 321 | 366 ([65 s capture](#cancel-after-a-rejected-keepuptodate-historical-request)) |
| pnl | 321 | 10185 |
| pnl_single | 321 | 10186 |
| scanner | 162 (disabled scan code) | 365 |
| contract_details, historical_ticks | 200 | none |
| historical_ticks | 10299 | none |
| account_summary | 321 | none |

- 354, 10168 and 10189 came from a CME future and a US stock without real-time permission. The
  same ES market data request drew 10168 in one run and 354 in the next, minutes apart.
- 420 on realtime bars (*live 2026-11-24 17:47-17:48 UTC, US market open, paper TWS, server
  223, market data types real-time and delayed*): IBM `TRADES` ("Invalid Real-time Query:No
  market data permissions for NYSE STK ..."), ESZ6 `TRADES` ("... for CME FUT") and EUR.USD
  `TRADES` ("... No historical market data for EUR/CASH@FXSUBPIP Last 0"). No bar followed for
  that request id, and the cancel the client sent drew 300 "Can't find EId with
  tickerId:1500000001" 1 to 2 ms later, under both types. The same session's EUR.USD `MIDPOINT`
  bars streamed. With the delayed type, ESZ6 drew 354 instead and MSFT drew 10089 under both
  types (420 at 16:42 UTC). Market data, tick-by-tick and keep-up-to-date requests on these
  contracts drew 10089, 354, 162 or 321, never 420.
- The reply has no recipient; the transport logs `no recipient for error` at INFO. It does not
  reach `notice_stream()`. Cost: one wasted write and one log line.
- A cancel that draws no reply proves nothing either way, so those types keep cancelling.
- `positions_multi` / `account_updates_multi`: TWS accepts a made-up account (DU0000000) with
  no error, so no terminal error could be provoked.
- 10314 came from a 1975 end date (IB rejects that old a date), not a format problem.
- Skipping the cancel is opt-in per verified code (`StreamDecoder::rejection_ends_request`). A
  global skip was rejected: a code we treat as terminal may not end the request at TWS (10197
  on market data; 10299 on historical bars, after which TWS sends the bars), and skipping would
  leak TWS resources.
- **Not probed:** 354 and 10168 on market data during RTH; 10197; market depth 354 on a stock.

### Cancel after a rejected keepUpToDate historical request

*Live 2026-11-26 16:21-16:33 UTC (US Thanksgiving holiday; FX open), paper TWS, server 223, raw
requests; first run 2026-11-25 09:57-10:01 UTC off-hours, same server.*

- Raw `reqHistoricalData` with `keepUpToDate` true. After the error the request id was watched
  for 65 s with no cancel sent, then `cancelHistoricalData` went out under the same id:

  | Error | Request | Frames in 65 s after it | Cancel reply |
  | ----- | ------- | ----------------------- | ------------ |
  | 200 "No security definition has been found for the request" | ZZZZQX stock | none | 366 within 1 ms |
  | 162 "...No market data permissions for IBIS STK" | SAP on IBIS, 1 h `TRADES`, 1 day | none | 366 within 2 ms |
  | 162 "...invalid step: 1" | EUR.USD, 1 s `MIDPOINT`, 2 days | none | 366 within 1 ms |
  | 162 "...HMDS query returned no data: ESZ8@CME Trades" | ESZ8 and ESH9, 1 min `TRADES`, 1800 S | none | 366 within 2 ms |
  | 162 "...No historical market data for EUR/CASH@FXSUBPIP Last 3600" | EUR.USD `TRADES`, 1 h, 1 day | none | 366 within 4 ms |
  | 321 "Error validating request.-'D' : cause - Source price not supported with live updates" | AAPL `ADJUSTED_LAST` | none | 366 within 1 ms |
  | 10299 "Expected what to show is TRADES ...", then the same 321 at once | AAPL `AGGTRADES` | none after the 321 | 366 within 2 ms |

  The 162 texts start "Historical Market Data Service error message:".
- A 366 alone does not show that TWS ended a request: a cancel sent within ~1 ms of a 10299 on
  a crypto `TRADES` request drew 366 while the updates kept coming. Hence the 65 s watch.
- The first run (cancel 1.5 s after the error, 366 within 1 ms each) also covered 200 on 005930
  (KSE), 321 "End date not supported with live updates" (EUR.USD with an end date, which
  `subscribe_with_updates()` rejects before sending) and 321 for BTC `AGGTRADES`. The same
  requests with `keepUpToDate` false drew the same 200 and 162 errors and the same 366.
- Requests that streamed answered the cancel with 162 "API historical data query cancelled:
  <id>" instead: EUR.USD `MIDPOINT` (updates every ~5 s), and ESZ6 (CME) and 7203 (TSE)
  `TRADES`, whose bars came without a permission error and then no update or error for 12 to
  45 s.
- Pacing violation: not provoked. Three identical requests (EUR.USD `MIDPOINT`, 1 h bars) 0.3 s
  apart, and then eight for the same contract with different durations 0.15 s apart, were all
  served.
- `HistoricalDataFrame::rejection_ends_request` matches 200 and 321 under `keep_up_to_date`. 162
  still cancels, since its pacing-violation form is unchecked.
- **Not probed:** a pacing-violation 162; 354 and 10182 on a historical request (no contract
  tried drew 354, and no HMDS farm disconnect happened); RTH.

### Cancel after a request TWS finished

*Live 2026-11-24 off-hours, paper TWS, server 223.*

- Snapshot market data (EUR.USD, BTC) ends with TickSnapshotEnd; no cancel is sent after it.
- historical_data `subscribe()` (EUR.USD, ES) ends with HistoricalDataEnd; no cancel.
- historical_ticks (EUR.USD midpoint) ends on the batch flagged done; no cancel.
- contract_details (AAPL) ends with ContractDataEnd; no cancel.
- Still-live requests answer the cancel without error: streaming market data, tick-by-tick and
  market depth on EUR.USD, account_summary after its End (the stream stays open), pnl. A
  `subscribe_with_updates()` historical request answers it with 162 "API historical data query
  cancelled".

### 10089 ends the request without an end marker

*Live 2026-11-24 off-hours, paper TWS, server 223, market data type real-time.*

- AAPL snapshot, AAPL streaming market data and AAPL realtime bars each drew 10089 "Requested
  market data requires additional subscription for API ... Delayed market data is available",
  then nothing for 12-15 s: no ticks, no TickSnapshotEnd. A cancel afterwards drew 300 every
  time, so TWS had already ended the request.
- The client classifies 10089 as a data advisory (`DATA_ADVISORY_CODES`), so the stream stays
  open and its drop still sends the cancel.
- With market data type delayed (3) or delayed-frozen (4), the same requests drew 10167 and
  delayed ticks; frozen (2) drew 10168.
- **Not probed:** RTH, and an account with delayed data enabled for the API.

### Cancel of a running historical bar request (162)

*Live 2026-11-05 off-hours, paper, server 226; #1167.*

- `historical_data(..).between(..).subscribe()` (keepUpToDate false, 300 days of hourly SPY
  bars, end date set), then `cancelHistoricalData` right after the request: within ~50 ms TWS
  sends Error 162 for the request id, "Historical Market Data Service error message:API
  historical data query cancelled: <id>". No bars and no HistoricalDataEnd. 3/3 runs, sync
  and async.
- So TWS confirms the cancel, and `cancel_and_drain` returns `Drained::Rejected` (162): the
  request is over at TWS. A cancel after the request already ended draws 366 instead (see
  [Cancel after a request-bound terminal error](#cancel-after-a-request-bound-terminal-error)).
- **Not probed:** a cancel that crosses bars already in flight, and RTH. Whether the 162 still
  follows when the bars were sent first is unverified.

### One-shot requests answer once

*Live 2026-10-26, paper Gateway, off-hours; #1107.*

- `calculate_option_price`, `calculate_implied_volatility`, `head_timestamp`,
  `histogram_data`, historical schedule: exactly one frame each, then nothing for 15s. Option
  computations do not stream.
- A cancel sent after the answer draws no error (option price, head timestamp, histogram):
  harmless, unneeded. The answer acts as a snapshot end.
- **Not probed:** WSH metadata (paper account gets 10276 "News feed is not allowed").

### End markers come after the initial dump only

*Live 2026-10-05, paper Gateway, single account, group "All"; PR #957 review. Diagnostics:
`integration/async/tests/end_markers.rs` (`#[ignore]`).*

- account_summary, account_updates, account_updates_multi, positions, positions_multi send End
  after the initial dump only. No End after fill-triggered pushes (~21s, ~61s) or the 3-minute
  periodic push (~241s). Never assume one End per push.
- **account_summary sends End twice** (group "All", 3/3 reproductions; recorder showed 1
  request, 2 AccountSummaryEnd frames on the wire, so not a routing bug):
  - regular tags only: rows, End, End
  - `$LEDGER:ALL` only: ledger block, End, the same ledger block, End
  - both: rows, ledger, End, ledger, End
  - One earlier 30s run showed 1 End, so not fully deterministic.
- account_summary pushes: 111 rows within one second; pushes are ≥40s apart. Pushes include
  unchanged values.
- **Open:** after `cancel().await` (cancel frame on the wire with the same id, as in C#), a
  third account_summary request on the same connection still got 322 (max 2) 10s later. A fresh
  connection per request works. Unknown whether TWS keeps the slot or the cancel is
  ineffective.

### Data-farm notices

*Live 2026-11-13 (session not recorded), paper TWS, server 223; live 2026-11-14 weekend, paper Gateway,
server 226, connect burst plus a 30-min session (Sat 15:24–15:54 ET) with a forex quote and
connectivity flaps; #1209 (`FARM_CODES`, `Notice::data_farm` in `src/messages/notice.rs`).*

- Every farm notice is global (request id -1) and carries the farm name in its text. All but
  the inactive notice put it after a `:` with no space:
  - connect burst, TWS server 223: `2104 Market data farm connection is OK:usopt`,
    `2104 ...OK:usfarm`, `2106 HMDS data farm connection is OK:ushmds`,
    `2158 Sec-def data farm connection is OK:secdefnj`.
  - connect burst, Gateway server 226: `2104 ...OK:usfuture` (earlier run only),
    `2104 ...OK:usfarm`, `2106 ...OK:ushmds`, `2158 ...OK:secdefnj`.
  - connecting, Gateway 226, on the first EUR.USD quote: `2119 Market data farm is
    connecting:cashfarm`, then `2104 ...OK:cashfarm` in the same second.
  - broken, Gateway 226, during weekend flaps: `2103 Market data farm connection is
    broken:usfarm`, `2105 HMDS data farm connection is broken:ushmds`, `2157 Sec-def data farm
    connection is broken:secdefnj`. Each flap ran broken → 1100 → OK codes → `1102 ...restored
    - data maintained. All data farms are connected: usfarm; ushmds; secdefnj.`; 1100 can come
    before or after the broken codes.
- Inactive uses a `.` instead: `2108 Market data farm connection is inactive but should be
  available upon demand.cashfarm`, about 2.5 min after the cashfarm quote was cancelled. It
  arrived as two identical frames (both in the raw capture, so TWS sends it twice).
  `Notice::data_farm` parsed every notice above to the right kind, status and name.
- **Not probed:** 2107 (HMDS inactive): `ushmds` stayed up through 30 min idle after one
  head-timestamp request. Non-OK codes on TWS (only Gateway showed them), and farms other than
  those named above.
