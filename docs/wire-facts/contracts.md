# Contracts wire facts

Contract details and option chains. Format: [README](README.md).

### `Contract.lastTradeDate` (tag 21)

*Live 2026-10-26, server 226, contract_details; #1099.*

- Plain `YYYYMMDD` for OPT, FUT and FOP; absent for STK and BOND.
- The `YYYYMMDD HH:MM:SS Zone` form appears only in tag 4 `lastTradeDateOrContractMonth`
  (FUT/FOP).
- C# never encodes tag 21, so `ContractBuilder::last_trade_date` has no wire effect.

### Option chain `exchange` is a futures-options filter

*Live 2026-09-03, paper Gateway, server 220, `reqSecDefOptParams`; #792.*

| exchange | contract_id | Result |
|---|---|---|
| `""` | 265598 (AAPL) | 39 chains across 20 exchanges |
| `SMART` / `CBOE` / `NASDAQOM` | 265598 | clean end, zero chains |
| any | 0 | 321 "Invalid contract id" |
| `""` | 756733 (SPY) | 39 chains |

- The field is TWS's `futFopExchange`. Stock underlyings must leave it unset; a named exchange
  returns an empty chain, silently.
- `contract_id` is required; "0 for underlying" was wrong.
- **Not probed:** a real FOP underlying (e.g. ES on CME).
