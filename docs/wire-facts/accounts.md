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

### A second `reqAccountUpdates` for the subscribed account draws no reply

*Live 2026-11-25 10:20-10:28 UTC off-hours, paper TWS, server 223, single account, sync.*

- `reqAccountUpdates(true, account)` gets the initial dump: 242 `AccountValue`, 12
  `PortfolioValue`, 13 `AccountUpdateTime`, then `AccountDownloadEnd`.
- The same request again on the same connection while the first is live, after its
  `AccountDownloadEnd`: no frame in the next 6 s. Two written back to back (the second before
  the first dump ends): one dump only.
- `reqAccountUpdates(false)` then `true` back to back while subscribed: TWS sends a request-less
  2100 "API client has been unsubscribed from account data.", then the full dump again with its
  `AccountDownloadEnd` (same counts). Written right after the first `true`, before its dump
  ended: two full dumps, each with its end marker.
- `false`, a 5 s wait, then `true`: 2100, nothing during the wait, then the full dump.
- `true` for another account code (`DU0000000`) while subscribed: no frame and no error; `true`
  for the subscribed account after it: no frame either, so TWS still counted that account as
  subscribed.
- So a second `account_updates` caller for the same account is sent as `false` then `true`;
  the dump that follows reaches every subscription of the account. Live with that change, the
  first caller read 242 / 12 / 13 and `End`, the second caller the same 100 ms after its
  request, and the first caller the dump and `End` again.
- **Not probed:** updates for the subscribed account after the other-account request (the
  account has no second account to switch to, and no change happened in the window), an FA
  account, RTH.
