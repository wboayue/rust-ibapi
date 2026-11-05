# Accounts wire facts

Positions and PnL. End-marker behavior for account streams is in
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
