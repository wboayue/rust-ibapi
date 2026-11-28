# Accounts wire facts

Positions, PnL and account updates by model. End-marker behavior for account streams is in
[protocol](protocol.md#end-markers-come-after-the-initial-dump-only). Format: [README](README.md).

### Flat position sends `"0"`

*Live 2026-10-26, paper Gateway, EUR.USD 25k buy then sell; #1088.*

- After closing, Position and PositionMulti frames carry `position = "0"`. TWS does not omit
  a zero quantity, so an absent `position` is not the flat case and decoding it as required
  does not end streams on a close.
- PnLSingle on the flat position also sent `"0"`.
- `pnl_single` for a never-held conid sends no frame at all (held ones get a 2150 notice
  first).
- **Not probed:** PortfolioValue (forex is not in the portfolio; needs STK in RTH).

### Unset PnL figures are omitted, not `f64::MAX`

*Live 2026-11-05 off-hours (16:51 ET), paper, server 226; #1170.*

- `pnl` on the account: the PnL frame carried all three figures (tags 2–4); `unrealizedPnL`
  was an explicit `0.0`.
- `pnl_single` on two flat futures positions (NQ, ES; `position = "0"`): each PnLSingle frame
  carried tags 1, 2, 3, 5, 6 and no tag 4, so `unrealizedPnL` is left out of the frame. `value`
  was an explicit `0.0`. Each frame came after a 2150 notice.
- No frame carried `f64::MAX`. The C# decoder substitutes `double.MaxValue` for a missing
  field (`HasUnrealizedPnL` false), so the sentinel C# callers see is the client's, not TWS's.
- **Not probed:** an open (non-flat) position, and RTH updates. Which figures TWS omits for a
  held position is unknown.

### `ledgerAndNLV = true` selects the ledger keys; a TWS setting decides the `$LEDGER-` prefix

*Live 2026-11-14 weekend (01:44 and 01:50 ET), paper, server 226; #1223.*

- `reqAccountUpdates` and `reqAccountUpdatesMulti` (one account, no model code, `ledgerAndNLV`
  `false` then `true`), each read to its end marker, run twice: once as the Gateway was, once
  after turning on Global Configuration → API → Settings → "Prepend '$LEDGER-' prefix to
  per-currency account values".
- `ledgerAndNLV = true`: 75 values, the same 25 ledger keys for each of BASE, EUR and USD
  (`CashBalance`, `AccruedCash`, `NetLiquidationByCurrency`, `ExchangeRate`, ...), and nothing
  else. `false`: those 75 plus the account-level keys, 216 values. Same counts with the
  setting off and on.
- Setting off: no key carries `$LEDGER-`. The ledger row and the account-level row of a key
  that is both collide: with `false`, `AccruedCash`/`USD` arrived twice, `31.99` (account
  level, equal to the BASE ledger row) and `212.85` (the USD ledger row). Key plus currency
  does not tell them apart.
- Setting on: exactly the 75 ledger rows carry `$LEDGER-` (`$LEDGER-AccruedCash`/`USD` =
  `212.85`); the account-level rows keep the bare key (`AccruedCash`/`USD` = `31.99`). BASE
  ledger rows are prefixed too.
- `reqAccountUpdates` is unaffected: 215 values, never a `$LEDGER-` key, and no duplicate
  key/currency pair under either setting.
- So the prefix is opt-in per Gateway/TWS install, not fixed protocol. The setting's default for
  new vs upgrading installs is IB's claim (10.47 release notes), not observed here.
- Earlier: 2026-11-05, a live account through a read-only Gateway (server 226, #1183) saw the
  prefix, so its setting was on; the `true`/`false` subset finding matched.
- **Not probed:** a model code, an FA master account, later update frames (only the initial
  dump to the end marker was read), whether the Gateway applies a changed setting to an
  already-open subscription (each run reconnected).

### A positions request during an internet outage gets 2151, then nothing

*Live 2026-11-25 and 2026-11-26 around 12:00 UTC (off-hours, US pre-market), paper TWS, server
223; internet outages of 15 s to 20 min that kept the API socket up (#1300, #1315, #1323).*

- `reqPositions` sent after the 1100 drew one 2151 with request id -1, then no `Position` and no
  `PositionEnd`. Nothing more came in 323 s, 1554 s and 348 s, and none came after the restore:

  ```text
  12:01:43.831 code=2151 "Positions info is not available yet." (request id -1)
  ```

- A positions request opened before the outage kept its subscription. TWS re-sent the 4 open
  positions on it, with no `PositionEnd`, 2.0 to 3.3 s before the 1102.
- Sent 0.1 s after the network returned, `reqPositions` got all 4 open positions and
  `PositionEnd`.
- `positions()` ends a subscription that is still waiting for its `PositionEnd` on a 2151, with
  `Err(Error::Notice)`. A subscription that already has its `PositionEnd` yields the 2151 as a
  `Notice` and stays open (`PositionUpdate::notice_fails_initial_batch`).
- **Not probed:** two positions requests pending at once (one 2151 or one each), including one
  still mid-dump when a newer request draws the 2151, which `positions()` also ends; whether a
  request sent after the 1102 completes on a client whose earlier request got 2151;
  `positions_multi` (which has a request id) during an outage.
