# Contracts wire facts

Contract details and option chains. Format: [README](README.md).

### Quarterly-only futures need `next_quarter()`, not `front_month()`

*Live 2026-11-03 22:39 ET off-hours, paper, server 226; #1149.*

- `Contract::futures("ES").front_month().on_exchange("CME")` asks for the current month
  (`ContractMonth::front` picks the current month through the 15th, then the next): November
  on 2026-11-03. ES lists only quarterly contracts, so `historical_schedules` drew error 200 "No
  security definition has been found for the request". `next_quarter()` (December) resolved.
- `Contract::futures("GC").front_month().on_exchange("COMEX")` resolved: GC lists nearby
  monthly contracts.
- **Not probed:** other quarterly-only products (NQ, ZN, 6E), and which month `front_month()`
  picks near a quarterly expiry.

### `Contract.lastTradeDate` (tag 21)

*Live 2026-10-26, server 226, contract_details; #1099.*

- Plain `YYYYMMDD` for OPT, FUT and FOP; absent for STK and BOND.
- The `YYYYMMDD HH:MM:SS Zone` form appears only in tag 4 `lastTradeDateOrContractMonth`
  (FUT/FOP).
- C# never encodes tag 21, so `ContractBuilder::last_trade_date` has no wire effect.

### `lastTradeDateOrContractMonth` (tag 4) on a future

*Live 2026-11-13 09:55 ET RTH, paper TWS, server 223; contract_details for ES 202612.*

- Tag 4 arrives as `20261218 08:30:00 US/Central`; `time_zone_id` is `US/Central` too.
- Sent back as the date alone or in that full form, `contract_details`, `head_timestamp`
  and a market-data snapshot all succeed: this build accepts both forms.
- **Paper Gateway accepts every form too** (*live 2026-11-14 14:15 ET off-hours, paper
  Gateway, server 226*): ES sent as `20261218 08:30:00 US/Central`, `20261218 08:30:00`,
  `20261218` and `202612` each resolved in `contract_details`, `head_timestamp`, a market-data
  snapshot and a what-if limit order (same margin for all four). No 10372.
- **Zone equals `time_zone_id`** (*live 2026-11-14 14:00 ET off-hours, paper Gateway, server
  226*): every contract carrying a zone in tag 4 matched its `time_zone_id`. FUT on CME (ES),
  NYMEX (CL), CBOT (ZN), CFE (VIX), EUREX (DAX, ESTX50 → `MET`), ICEEU (Z → `GB-Eire`),
  OSE.JPN (N225 → `Japan`), HKFE (HSI → `Hongkong`); FOP ES 202612 (6943 contracts); OPT SPX
  (`US/Central`) and SAP on EUREX. AAPL OPT on SMART sends the date alone (`20261218`), no time
  or zone. So dropping the zone loses nothing.
- **Not probed:** the gateway build behind server 226 (the handshake doesn't report it), so
  an NT adapter note of 10372 on Gateway 10.51 is neither confirmed nor ruled out; live
  (non-what-if) orders; bonds.

### Open-order echo of `lastTradeDateOrContractMonth` (tag 4)

*Live 2026-11-14 weekend, paper Gateway, server 226; #1214.*

- Buy limits far from the market, read back with `open_orders`. ES on CME sent as `202612`,
  as `20261218` and as `20261218 08:30:00 US/Central` all came back as `20261218` (conid
  515416632, PreSubmitted). AAPL OPT sent as `20261218` came back as `20261218`.
- So TWS normalizes tag 4 to the last trading day: a contract sent as a month comes back as a
  date, and the open-order echo carries no time or zone. `Expiry` therefore doesn't promise the
  variant round-trips.
- **Not probed:** positions and portfolio (the account held no FUT/OPT), executions (need a
  fill), and live TWS.

### Bond contract details on a paper account

*Live 2026-11-14 weekend, paper Gateway, server 226; #1214.*

- `Contract::bond_cusip("912810TW8")` (UST 4¾ 11/15/43) → one contract (conid 667573987) with
  tag 4, symbol, currency, exchange, `maturity` and `coupon` all empty. Only `contract_month`
  `204311`, `desc_append` `T 4 3/4 11/15/43`, the CUSIP/ISIN, trading class `US-T` and
  `time_zone_id` `US/Eastern` were filled. It looks permission-stripped.
- Re-querying by conid resolves. With conid 0 the query draws 200, even with the fields as
  returned.
- `UKT`/GBP bonds → 190 contracts, all with tag 4 empty.
- So the zoned bond form (`"20430504 16:00 Europe/London"`, from C#'s `isBond` split) was not
  seen here, and whether a bond sent back with the date alone resolves is unanswered. The
  decoder leaves a bond's tag 4 as sent (`Expiry::Unknown` when zoned), as C# does, so nothing
  depends on the answer.
- **Not probed:** an account with bond permissions.

### `tradingHours` and `liquidHours` sessions

*Live 2026-11-13 09:55 ET RTH, paper TWS, server 223; contract_details for ES 202612.*

- Both fields arrive as `;`-separated entries, each `YYYYMMDD:HHMM-YYYYMMDD:HHMM` or
  `YYYYMMDD:CLOSED`, covering about a week: `20261112:1700-20261113:1600;20261114:CLOSED;...`.
- The close carries its own date, so an overnight session (17:00 to 16:00) needs no rule. The
  pre-970 form (`20090507:0700-1830,1830-2330`) did not appear.
- `liquidHours` lists a closed day the full session still trades (`20261115:CLOSED` beside
  `20261115:1700-20261116:1600` in `tradingHours`) and can hold two sessions for one date
  (`20261118:0830-20261118:1600;20261118:1700-20261119:1600`).
- Times are wall clock in `timeZoneId` (`US/Central` here).
- *Live 2026-11-15 15:42 ET (Sunday), paper Gateway, server 226; AAPL STK SMART, EUR.USD CASH
  IDEALPRO, SPX OPT CBOE 20261120 (1014 contracts), SAP STK IBIS EUR:* every entry is one of the
  two forms; `TradingSession` parsed all of them. EUR.USD (both fields) and SPX `tradingHours`
  start with an empty entry (`;20261115:1715-20261116:1700;...`), so a parser must skip empty
  items. SPX `tradingHours` holds two sessions per date (`1915-0825` overnight, `0830-1600`);
  `timeZoneId` was `US/Eastern`, `US/Central` (SPX) and `MET` (SAP), not always IANA.
- **Not probed:** futures options, bonds, and products whose `timeZoneId` differs from the
  exchange's.

### Price magnifier units

*Live 2026-11-23 18:50-19:10 ET off-hours, paper TWS, server 223; contract_details,
option_chain, delayed-frozen market data, historical bars, calculate_option_price.*

`priceMagnifier` by product:

| Magnifier | Contracts |
| --- | --- |
| `100` | FUT ZC, ZS, ZW, ZL (CBOT), HE, LE (CME), KC, SB (NYBOT); FOP ZC; STK VOD, BARC, HSBA (LSE); OPT VOD (ICEEU); FUT and IND Z (ICEEU) |
| `1` | FUT ES, GC, CL, ZM, N225, HSI, FESX, FDAX, R; FOP ES; OPT AAPL, SPX, SAP, 9984; STK AAPL, 7203 (TSEJ), 700 (SEHK), SAP; CASH EUR.USD; IND SPX, N225; CFD AAPL |

No contract sent `0` or a negative value.

Units for the magnified contracts probed (ZC FUT 202703, ZC OPT OZC Jan 2027 450 C, VOD
STK LSE, VOD OPT ICEEU Dec 2026 100 C, Z FUT 202612):

| Value | Unit | ZC future / ZC option | VOD stock / VOD option |
| --- | --- | --- | --- |
| Market data bid, ask, last, close | sub-unit | `449` / `7.875` | `102.85` / `6.25` |
| Historical bar prices | sub-unit | `449` / `7.75` | `103` / 200 with the details strike `1` |
| `option_chain` strikes | sub-unit | `360`, `365`, ... | `50`, `60`, ... |
| Strike TWS accepts in a request | sub-unit | `450` (`4.5` draws 200) | `120` (`1.2` draws 200) |
| Strike in contract details | currency | `4.5` | `1.2`, `1` |
| Option computation `optPrice` | currency | `0.07875` | `0.0525` |
| Option computation `undPrice` | currency | `4.4875` | `1.0293` |
| `calculateOptionPrice` underlying price | currency | `4.49` gives `0.0767`; `449` treats the option as deep in the money | not tried |
| `minTick` | currency (ZC's quarter-cent tick is `0.0025`) | `0.0025` / `0.00125` | `0.000001` / `0.0025` |

- Price × multiplier / magnifier gives the notional value: ZC `449` × `5000` / `100` =
  $22,450. Z on ICEEU quotes index points (`10524`) with magnifier `100` and multiplier `1000`,
  so its sub-unit is not a currency unit.
- The contract details return for the ZC option (conid 896736796, strike `4.5`) drew 200 from
  `historical_data`. The same contract with strike `450`, strike `0` or the contract id alone
  returned bars. A delayed market-data request accepted it with strike `4.5`.
- IB's documentation says the magnifier is not used for futures-option strikes from TWS 972.
  That holds only for the strike in contract details.
- **Not probed:** orders and executions (no orders placed), HKD and JPY products with a
  magnifier above 1 (none found), real-time market data (no CME or LSE subscription; values
  are delayed-frozen).

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

### `underSecType` is a security type code

*Live 2026-11-16 Mon 02:30 ET (US equities closed, CME Globex open), paper, server 223.*

- `reqContractDetails`: AAPL stock and SPX index send none (empty); the ES future sends `IND`;
  680 AAPL options for 202612 all send `STK`; 7582 ES futures options for 202612 all send `FUT`.
- Settles typing `ContractDetails::under_security_type` as an optional `SecurityType`.
- **Not probed:** bonds, warrants, CFDs, forex, and futures on other underlyings.

### Matching-symbols `derivativeSecTypes` are security type codes

*Live 2026-11-22 Sun 23:54 ET (markets closed), paper, server 226; #1261.*

- `reqMatchingSymbols` for 30 patterns (AAPL, SPX, ES, EUR, BTC, GOLD, SAP, DAX, ZN, CL, HSI,
  7203, and others): 1330 descriptions across STK, IND, BOND, FUND, CASH, CMDTY and CRYPTO.
  953 carry an empty list; the rest carry only `BAG`, `CFD`, `EC`, `FOP`, `FUT`, `IOPT`, `OPT` and
  `WAR`. No entry was an empty string.
- `IOPT` appeared on 93 descriptions (68 STK, 23 IND, 2 CASH); `EC` on 12, all IND, among them
  event-contract indices such as `KXBTCY` and `KXGOLDD`. #1261 models both as `SecurityType`
  variants.
- Settles typing `ContractDescription::derivative_security_types` as `Vec<SecurityType>`.
- **Not probed:** patterns outside this list; only one session; whether TWS accepts `IOPT` or `EC`
  as a request contract's `secType`.

### Option chain expirations are `YYYYMMDD` dates

*Live 2026-11-16 Mon 02:30 ET (US equities closed, CME Globex open), paper, server 223; #1253.*

- `reqSecDefOptParams` for AAPL (22 chains), SPX (9 chains) and ES futures options on `CME`
  (21 chains, one per trading class such as `EW1`, `E3B`): every `expirations` entry is an
  8-digit `YYYYMMDD` date, weeklies and dailies included.
- Settles typing `OptionChain::expirations` as dates. No contract month (`YYYYMM`) form appeared.
- **Not probed:** bond or warrant chains; other futures roots.

### Market rule ids align with valid exchanges

*Live 2026-11-10 RTH, paper Gateway, server 226; #1198.*

- `contract_details` for STK (AAPL SMART and any exchange, SAP@IBIS, 7203@TSEJ), FUT (ES,
  GC), OPT (SPY 20261120), FOP (ES 202612), CASH, CRYPTO, IND, CFD, CMDTY and FUND: 5,834
  rows, 16,598 `marketRuleIds` tokens. Every token is a plain integer: no whitespace, no
  empty entries.
- In every row the id count equals the `validExchanges` count: id *i* is the rule for
  exchange *i*, as IB's `ContractDetails` doc says. Usually all ids are equal (AAPL: 4563
  ×27); some contracts mix them (SAP@IBIS: 1874, 1943, 3055; 7203@TSEJ: 3407, 1460).
- **Not probed:** bonds, warrants, structured products; contracts with no market rule.

### Event-contract, `minAlgoSize` and precision fields

*Live 2026-11-15 Sun 23:52 ET (off-hours), paper, server 226; raw capture of `ContractData`; #1251.*

| Contract | `eventContract1` / `Description1` / `Description2` | `minAlgoSize` | `lastPricePrecision` / `lastSizePrecision` |
|---|---|---|---|
| AAPL STK | absent | `"0"` | `"0.000001"` / `"0.000001"` |
| ES FUT (front month) | absent | `"0"` | absent |
| EUR.USD CASH | absent | `"0"` | absent |
| BTC CRYPTO (PAXOS) | absent | `"0"` | absent |
| FF OPT on FORECASTX (62 rows) | `"US Fed Funds Target Rate"` / `"FF Jan28'27 3.125"` / `"Will the US Fed Funds Target Rate be set above 3.125% at the FOMC meeting ending January 27, 2027?"` | `"0"` | absent |

- The event-contract strings are present and meaningful only on event contracts.
- `minAlgoSize` was `"0"` on every row, including AAPL, which takes algo orders, so no
  contract probed carried a usable value.
- The precision fields are steps (`0.000001`), not digit counts, and only AAPL sent them.
  `TickReqParams` matched ([market data](market-data.md#tickreqparams-precision-fields)).
- **Not probed:** equity and index options, bonds, funds, non-US stocks; a contract with a non-zero
  `minAlgoSize`; what the AAPL step governs (sub-penny prices and fractional shares are both
  smaller than its `minTick` 0.01 and `minSize` 0.0001).
