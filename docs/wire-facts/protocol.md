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

*Live 2026-10-26, paper Gateway; #1094, #1097, #1096.*

TWS reply to a cancel sent after the request already failed. Observed while every type still
sent the safety-net cancel; types whose code is verified here now skip it, so the reply no
longer occurs for them.

| Request | Terminal error | Cancel reply |
|---|---|---|
| market_data, realtime_bars, tick_by_tick | 200 | 300 |
| market_depth | 200 | 310 |
| historical_data (stream / fetch) | 200, 162, 10314 | 366 "No historical data query found" |
| pnl | 321 | 10185 |
| pnl_single | 321 | 10186 |
| scanner | 162 (disabled scan code) | 365 |
| contract_details, historical_ticks | 200 | none |
| account_summary | 321 | none |

- The reply has no recipient; the transport logs `no recipient for error` at INFO. It does not
  reach `notice_stream()`. Cost: one wasted write and one log line.
- `positions_multi` / `account_updates_multi`: TWS accepts a made-up account (DU0000000) with
  no error, so no terminal error could be provoked.
- 10314 came from a 1975 end date (IB rejects that old a date), not a format problem.
- Skipping the cancel is opt-in per verified code (`StreamDecoder::rejection_ends_request`). A
  global skip was rejected: a code we treat as terminal may not end the request at TWS (10197
  market data), and skipping would leak TWS resources.

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

### A TWS stall leaves historical, contract-detail and completed-order requests unanswered

*Live 2026-11-24 23:06 to 2026-11-25 06:19 UTC, paper TWS, server 223; ended by a TWS restart.*

| Request | During the stall | After the restart (06:21) |
| ------- | ---------------- | ------------------------- |
| `contract_details` OPT 700 SEHK | 23:06: no reply, no error in 170 s | 139 rows in 1 s |
| `completed_orders` (`api_only` true and false) | 23:09-23:10: nothing in 10 s, three times | 8 orders at once |
| `historical_data`, keep-up-to-date or not (700 SEHK, HSI future, EUR.USD) | from 00:55: no batch, no update, no error, on new connections too, checked 10 times until 06:19 | batch and updates within 1 s |
| `reqMktData`, `realtime_bars`, orders and cancels | answered throughout | answered |

- The N225 historical request at 23:30 still answered, so the historical service stopped
  between 23:30 and 00:55. At 00:56:40-00:57:51 the farm notices reported `usfuture`, `ushmds`,
  `cashhmds`, `usfarm`, `secdefnj`, `cashfarm`, `usopt` and `uscrypto` broken and then OK again;
  no notice marked the stall itself or its end.
- New connections took the handshake as usual, except some that timed out after 30 s ("no
  startup reply from TWS") when made seconds after another connection closed (22:04 twice,
  01:36, 04:05).
- A subscription opened during the stall is created normally and then stays silent, which on a
  keep-up-to-date stream looks like a session break, and a one-shot request without a timeout
  waits for good. Only a watchdog on the caller's side detects it.
- Whether long-held keep-up-to-date streams caused it is not settled: the probe's own streams
  (up to four at once, held 1.5 h, 21:41-23:20) had all ended by 00:40, before the historical
  service stopped, but other clients on the same TWS ran their own requests.
- **Not probed:** the cause; IB Gateway; whether a reconnect of every client, short of a restart,
  clears it.
