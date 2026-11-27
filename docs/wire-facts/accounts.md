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

### Account streams across an internet outage

*Live 2026-11-25 and 2026-11-26, paper TWS, server 223; Wi-Fi off for 15 s, 2 min, 6 min and
20 min, router uplink unplugged for 2 min; #1300.*

- Account summary, positions and PnL resumed on the same subscription with no action. Positions
  re-sent the 4 open positions, no `PositionEnd`, 2.0 to 3.3 s before the 1102; account summary
  pushed 0.1 to 1.9 s after it.
- PnL pushed twice during the 6-minute outage. At restore one client's stream carried one value
  far off (unrealized -63,090 between values near -86,000); a `reqPnL` sent at restore first
  answered all zeros.
- 2026-11-26: at the restore every PnL stream, old or new, carried one item with all values zero,
  then normal values within 1 s (2 of 2 runs). Skip a lone zero item after the 1102:

  ```text
  12:34:01.191 [pnl/raw] data n=1 last=daily=Some(0.0) unrealized=Some(0.0) realized=Some(0.0)
  12:34:02.192 [pnl/raw] data n=1 last=daily=Some(16966.52181) unrealized=Some(-21835.02651958752) realized=Some(-227.25718285)
  ```

- `reqAccountUpdates` resumed on the same subscription 2.1 s before the 1102, positions 2.2 s
  before it, account summary 0.1 s after it.
- A `reqPositions` sent after the 1100 never ended: one global 2151 "Positions info is not
  available yet." and nothing more in 1554 s and 348 s, through the restore; see
  [Requests held during an internet outage](protocol.md#requests-held-during-an-internet-outage).
- **Not probed:** model variants; a positions change during the outage.
