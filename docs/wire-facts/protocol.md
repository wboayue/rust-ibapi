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
  global skip was rejected: a code we treat as terminal may not end the request at TWS, and
  skipping would leak TWS resources. 10197 does not end a market data request at all (see
  [Streams across an internet outage](market-data.md#streams-across-an-internet-outage)), and
  after 10299 on historical bars TWS sends the bars.
- 10182 on a keep-up-to-date bar request (paper TWS, server 223, 2026-11-26 outage runs): the
  cancel drew 366 within 3 ms in 4 of 4 cases, and no frame followed a 10182 in 1587 s and 390 s.
  `HistoricalDataFrame` skips that cancel.

  ```text
  12:01:30.656 [10182] Failed to request live updates (disconnected).   (request 1500000003)
  12:01:30.659 -> cancelHistoricalData 1500000003
  12:01:30.660 no recipient for error (id=1500000003): [366] No historical data query found for ticker id:1500000003
  ```

- **Not probed:** 354 and 10168 on market data during RTH; the cancel reply after 10197; market
  depth 354 on a stock.

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
- `HistoricalDataFrame::rejection_ends_request` matches 200, 321 and 10182 (above) under
  `keep_up_to_date`. 162 still cancels, since its pacing-violation form is unchecked.
- **Not probed:** a pacing-violation 162; 354 on a historical request (no contract tried drew
  354); RTH.

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
- TWS server 223 also sends the broken codes 2103, 2105 and 2157, seen in the 2026-11-25 outage
  runs (see [Internet outage: notices and timing](#internet-outage-notices-and-timing)).
- **Not probed:** 2107 (HMDS inactive): `ushmds` stayed up through 30 min idle after one
  head-timestamp request. Farms other than those named above.

### Internet outage: notices and timing

*Live 2026-11-25 12:40-13:08 UTC and 2026-11-26 12:00-12:35 UTC (US pre-market, FX open), paper
TWS, server 223; Wi-Fi off for 15 s, 2 min, 6 min and 20 min, router uplink unplugged for 2 min,
four API clients connected; #1300.*

- The API socket stayed up in all five runs: no transport reconnect, no connection-lost error.
- Loss: 1100 and broken-farm notices (2103, 2105, 2157) for every connected farm; no 2110. 1100
  came 17.5 s (as Wi-Fi returned), 36.5 s and 131.5 s after the cut; the first broken notice
  17.5 s, 50.3 s and 44.3 s. In the 6-minute run broken notices came 87 s before the 1100.
- With the 1100, TWS sent 1104 "Pending to create 1 orders: <order id>" to the client that had
  placed an order during the outage.
- Some farms went to 2108 instead of, or after, broken. jfarm went broken, then 2108, and got no
  2104.
- Restore order, every run: in the 2- and 6-minute runs a second 1100 23.7 s and 31.2 s after the
  network returned; then OKs for every farm within 1.3 to 2.1 s, including farms never reported
  broken; then 1102, 0.7 to 2.1 s after the first OK and 6.4 s, 28.5 s and 35.4 s after the
  network. Both HMDS OKs (2106 `ushmds`, `cashhmds`) came before the 1102.
- The 1102 farm list named 11, 8 and 6 farms while 11, 12 and 12 sent an OK. Do not parse it.
- 2026-11-26: 1100 came 24.3 s after the cut (Wi-Fi, 20 minutes) and 30.2 s (uplink); broken
  notices 33 to 42 s and 35 to 36 s after it. At restore the farm OKs came 12.7 to 15.8 s and the
  1102 14.4 s and 16.3 s after the network returned. Only the Wi-Fi restore brought a second 1100
  (11.3 s in). The 20-minute outage still ended in 1102, not 1101:

  ```text
  12:01:12.622 code=1100 "Connectivity between IBKR and Trader Workstation has been lost."
  12:21:09.442 code=1100 (second 1100)
  12:21:12.589 code=1102 "... restored - data maintained. All data farms are connected: cashfarm; usopt; usfarm; cashhmds; ushmds; secdefnj."
  ```

- Farm notices arrived only with request id -1.
- Requests sent during the outage were held, or rejected with 200 in a batch: 4 at 12:01:14.620,
  2.0 s after the 1100, and 8 at 12:32:37.955, 21.9 s after it. Cancels of held requests drew
  300, 310 or 366 (351, 112 and 66 such lines in the 20-minute run), and TWS could still send 200
  on the same id afterwards (1500000006, after its cancel had drawn 300). A 200 sent during an
  outage does not show the contract is invalid.
- Earlier, 2026-11-25: streaming requests sent during the outage got no frame and no error, for
  up to 354 s.
- `Client::resubscribing` waits for the HMDS OKs after a 10182 (`src/transport/farm_watch.rs`).
- `Client::resubscribing` reopens a stream on 1101, cancel first, on IB's documented rule alone.
- **Not probed:** 1101 (not seen in 20 minutes), so also whether TWS answers a cancel for a
  request opened before a 1101 (with 300, or at all) and whether such a request resumes on its
  own; an outage that drops the API socket; IB Gateway. A TWS restart is in
  [TWS restart and reconnect](#tws-restart-and-reconnect).

### Requests held during an internet outage

*Live 2026-11-25, paper TWS, server 223; same runs.*

- Historical bar requests sent during the outage were answered 0.4 to 0.5 s after the 1102
  (waits up to 146 s). Each reply ended at the minute the request was sent.
- `reqOpenOrders` sent 65 s in (after the 1100) was answered 82.6 s later. Before TWS noticed the
  outage it was answered at once.
- `reqPositions` sent 65 s in drew 2151 "Positions info is not available yet." with request id
  -1 and then nothing, also after the restore (open 323 s). Sent 0.1 s after the network
  returned, it got 4 positions and `PositionEnd` where other requests got 11 to 14.
- 2026-11-26, requests sent 1 s after the 1100: `reqPositions` drew one 2151 with request id -1
  and nothing more in 1554 s and 348 s, through the restore; `reqOpenOrders` was held 1196.9 s
  and 103.2 s; contract details were answered 0.1 s after the 1102; completed orders were
  answered once; historical bar requests 0.3 to 0.6 s after the 1102. `positions()` documents
  the bound a caller needs.

  ```text
  12:01:43.831 code=2151 "Positions info is not available yet." (request id -1)
  ```

- **Not probed:** whether a positions request sent after the 1102 completes on a client whose
  earlier request got 2151.

### TWS restart and reconnect

*Live 2026-11-26 11:06-11:31 UTC, paper TWS, server 223; two automatic TWS restarts, four API
clients with the library's reconnect, one adapter client with `max_reconnect_attempts = 200`;
#1300.*

- TWS closed the socket with EOF, then refused the port for 11 to 19 s. The first accepted
  connection got notice 10141 during the handshake and TWS reset the socket 0.5 s later. The
  next attempt, at most 15 s later, connected. No disclaimer dialog was shown and nobody clicked:

  ```text
  11:12:00.913 Connection error detected, attempting to reconnect: Io(Custom { kind: UnexpectedEof, error: "early eof" })
  11:12:01.915 reconnection attempt 1/20 failed: Connection refused (os error 61)
  11:12:11.923 reconnection attempt 4/20 failed: Connection refused (os error 61)
  11:12:21.416 [10141] Paper trading disclaimer must first be accepted for API connection.
  11:12:21.917 reconnection attempt 5/20 failed while establishing session: Connection reset by peer (os error 54)
  11:12:34.941 reconnected
  ```

- Downtime was 34.0 s (6 attempts, one 10141 per client) and 19.1 s (5 attempts, no 10141 seen;
  any 10141 phase fell inside the 8 s backoff). Both fit the default budget of 20 attempts.
- After the reconnect came farm OKs, once also 2103 `usopt` broken with the handshake. TWS
  re-sent the open orders 1.9 s after the reconnect, unrequested, and `next_valid_id` had
  advanced (51 to 53). Resting orders survived both restarts.
- Plain subscriptions ended with `ConnectionReset` and did not return; streams in
  `Client::resubscribing` reopened behind one -8 notice each, with first data 0.3 to 1.6 s
  later. Realtime bars missed during the restart were not back-filled (4 slots lost).
- The library retries a refused or reset connection only in its reconnect loop; the first
  connect fails on it.
- **Not probed:** a restart that needs a login or a manual quit, how long 10141 lasts then, a
  first connect during the 10141 phase, IB Gateway.
