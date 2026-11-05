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

### `ledgerAndNLV = true` selects the `$LEDGER-` keys only

*Live 2026-11-05 off-hours, live account through a read-only Gateway, server 226; #1183.*

- `reqAccountUpdatesMulti` for one account, no model code, once with `ledgerAndNLV = false`
  and once with `true`, reading to `AccountUpdateMultiEnd` each time.
- `true`: 75 values, the 25 `$LEDGER-*` keys for each of BASE, GBP and USD
  (`$LEDGER-CashBalance`, `$LEDGER-NetLiquidationByCurrency`, `$LEDGER-UnrealizedPnL`, ...) and
  nothing else. `false`: the same 75 plus 72 values of 70 account-value keys with no `$LEDGER-`
  prefix (`AccountCode`, `NetLiquidation`, `BuyingPower`, `AvailableFunds`, the `-S` segment
  variants, ...), 147 values. The `true` set is a strict subset of the `false` set.
- So `true` means "the ledger and NLV-by-currency view only", the light-weight request IB's doc
  page describes, and `false` adds the account values. The proto field comment ("Set as True if
  currency and net liquidation values should be delivered") names the subset, not an addition.
- **Not probed:** a model code, an FA master account, whether later update frames differ by
  flag (only the initial dump to the end marker was read).
