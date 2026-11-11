# Orders wire facts

Placement, echoes, status, executions and completed orders. Format: [README](README.md).

### `transmit` absent means held

*Live 2026-09-23, paper Gateway, server 220; #841 (closed, not a bug).*

- A placeOrder proto with `transmit` left out (what `some_bool(false)` produces): TWS holds the
  order. permId assigned, no OrderStatus, absent from open_orders; cancel returns 202 "Order was
  discarded" + 161. Control with `transmit=true` goes PreSubmitted → Submitted.
- The official clients (`EClientUtils` in C#/Java/Python/C++) also set `transmit` only when
  true. Our decoder's `unwrap_or(true)` applies to inbound messages only and says nothing about
  how TWS reads an absent flag in outbound ones.
- An untransmitted order gets no open-order or status frames, so a TWS-derived value (e.g. a
  VOL order's limit) is only visible once transmitted.

### Attached orders (`attached_orders`)

*Live 2026-09-27, paper Gateway, server 220; #842.*

- `slOrderType` / `ptOrderType`: only `"PRESET"` accepted. `STP`, `LMT`, `TRAIL`, `""` or junk
  → 320 "Invalid value for Stop Loss order-id or order-type" (text always says Stop Loss).
- Id without type, or type without id → 320. Each leg needs both.
- After a 320 the parent is never created.
- `PRESET` with no preset defined → 10355 "Cannot auto-attach … Preset is not defined.", then
  OrderStatus Cancelled + 202 "Order was discarded". The id stays consumed (103 on re-send).
- `transmit=false` does not defer the preset check.
- **Not probed:** child frames and parent-cancel behavior. The paper account has no presets;
  they are defined in TWS Global Config → Presets, and Gateway has no UI for them.

### `adjustable_trailing_unit`

*Live 2026-10-04, paper Gateway, AAPL parent LMT + attached adjustable STP → TRAIL; #901.*

- 0 (amount) and 100 (percent) accepted and echoed unchanged.
- 1 → 201 "Order rejected - Invalid value in field # 6269". 50 → 364 "Invalid Adjusted
  Trailing Amount".
- C# `Order.cs` documents percent as 1; wrong. The OrderSamples comments (100) are right.

### What-if and additional order params

*Live 2026-10-27 RTH, paper Gateway, server 226, SPY BUY LMT SMART DAY; #1065.*

- `what_if_type` without `what_if` → 321 "What-If type specified but What-If flag is not set";
  nothing placed. (Docs shipped in 5.3.0 said the order is placed live; corrected.)
- None / FullOrderCheck(0) / MarginRequirements(1) all echo `what_if_type =
  MarginRequirements` with the same OrderState. No observable effect on a stock what-if. The
  encoder does send 0.
- `post_only`, `allow_pre_open`, `seek_price_improvement` echo back as sent.
- `ignore_open_auction=true` echoes as false (sent, but not reported for SMART DAY LMT).
- `seek_price_improvement` unset → open order reports `Some(false)`: TWS sends an explicit
  false.

### IBKRATS PEG BEST / PEG MID

*Live 2026-10-04 (Sunday), paper Gateway, AAPL on IBKRATS, BUY 100 @ $1; #936.*

- Gateway first blocked every IBKRATS order with 10311 "This order will be directly routed to
  IBKRATS". `advanced_error_override="10311"` did not bypass it; changing the gateway's API
  precaution setting did.
- `not_held: false` → status Inactive, no error or warning. Required.
- `min_trade_qty` (1 or 100, the C# sample value) → 10302 "Min trade quantity is not allowed".
- Accepted (PreSubmitted) without `min_compete_size`, `compete_against_best_offset`, mid
  offsets (also with the +inf up-to-mid sentinel), `limit_price`; PEG MID with a stray
  `aux_price = 0.0` too.
- PEG BEST on SMART with `not_held` → 10297 "Not Held attribute is invalid for this order".
- **Caveat:** weekend PreSubmitted is not validation at the open.

### VOL order volatility is percent

*Live 2026-10-27 RTH, paper Gateway, server 226, SPY 20261030 calls; #1112.*

- BUY VOL 0.1 Annual on C770 (SPY ~768) → TWS-derived limit 0.01. A 10% vol would price ~2.12,
  which implies 0.1 means 0.1%.
- Contrast `calculate_option_price`, which takes a fraction: [market-data](market-data.md#option-computation-volatility-is-a-fraction).

### Combo (BAG) orders

*Live 2026-10-29, paper Gateway, server 226, AAPL options, what-if via `.analyze()`; #1123,
#1131, #1132. Guard: `integration/*/tests/spreads.rs`.*

- BAG with no `symbol` → 321 "The symbol or the local-symbol or the security id must be
  entered". Hence `Contract::spread(symbol)` requires it.
- Iron condor bought to open is a credit: Buy @ -1.00 accepted. Buy @ +1 → 201
  "Guaranteed-to-Lose"; Sell @ +1 → 201 "Riskless". TWS rejects mis-signed combo prices.
- Collar (STK leg ratio 100 + long put + short call) accepted, limit ≈ 100 × spot. Unlike
  option-only combos, no 201 bounds: prices below the put floor and above the call cap were
  all accepted.
- What-if answers outside RTH; all 8 `SpreadBuilder` presets PreSubmitted at 23:55 ET.

### `use_price_mgmt_algo` / `route_marketable_to_bbo` echo and acceptance

*Live 2026-11-05 off-hours (23:43 ET), paper Gateway, server 226; AAPL BUY 1 LMT 1.00; #1191.*

- Sent each field absent / 0 / 1, one at a time. Every `openOrder` echo carried both fields
  set: absent and 0 both came back as 0, so `decode_order` yields `Some(_)` and an echo
  never shows `None`.
- `use_price_mgmt_algo`: absent and 0 accepted (PreSubmitted, echo 0); 1 accepted (echo 1).
  This account's default is off.
- `route_marketable_to_bbo`: absent and 0 accepted (echo 0); 1 rejected with 10353
  ("...can result in higher total cost..."), "Restriction is specified in Precautionary
  Settings of Global Configuration/Presets"; the order was discarded (cancel drew 201).
- **Not probed:** an account whose default has either feature on (does 0 override it?);
  `route_marketable_to_bbo=1` with the precautionary setting relaxed; RTH routing effect;
  live TWS (Gateway only); servers below 226.

### OrderStatus `filled` / `remaining` always sent

*Live 2026-10-26, paper Gateway, EUR.USD; #1089.*

- Present in every state: PreSubmitted, Submitted, Cancelled carry `filled="0"`; Filled
  carries `remaining="0"`. TWS does not drop zero decimal strings, so both decode as required.

### Completed order `totalQuantity` is the unfilled quantity

*Live 2026-10-27, server 226, raw capture of CompletedOrder frames; #1117.*

| Order | totalQuantity | filledQuantity | status |
|---|---|---|---|
| SPY 1, filled | "0" | "1" | Filled |
| EWN 1000, filled | "0" | "1000" | Filled |
| EWN 1000 LMT, cancelled after 600 filled | "400" | "600" | Cancelled |
| cancelled, unfilled | original | "0" | Cancelled |

- Original size = total + filled. Live OrderStatus agrees (remaining 400).
- Paper partial fills: a limit BUY 1000 EWN at the ask filled in chunks (300/105/595) over
  ~10s. Late fills can arrive with the cancel reply; re-read `filled` from it.
- Bracket sibling-cancel (#1030) confirmed the same day; a fill without a Filled status never
  appeared in 8 fills.

### `open_orders` excludes finished orders

*Live 2026-10-18, paper Gateway, server 220; #1020.*

- After order 1 was cancelled (Cancelled status seen), `open_orders()` returned only working
  order 2, on the same connection and on a fresh one with the same client id. Finished orders
  are reachable only through `completed_orders`.
- **Not probed:** filled orders (assumed the same).

### Executions: `last_n_days` and `specific_dates`

*Live 2026-09-27 (Sunday), paper Gateway, server 220; #858.*

- Default `ExecutionFilter` → today only (Sunday: empty).
- `last_n_days = N` counts calendar days including today: on Sunday N=1,2 → empty; N=3 →
  Friday; N=7 → Tue–Fri.
- `specific_dates = ["yyyymmdd"]` → exactly that day.
- One commission report per execution comes back alongside.

### `exercise_options` account

*Live 2026-10-04, paper Gateway, AAPL call, no position held; #901.*

- Account `""`, the managed account, and a bogus `DU0000000` all returned 322 "No unlapsed
  position exists in this option in account <X>"; for `""` X was the login's account. Account
  is optional on a single-account login.
- **Not probed:** multi-account (FA) logins.

### `ext_operator` and `manual_order_indicator` on orders and cancels

*Live 2026-11-10 20:03–20:07 ET (AAPL off-hours, ES Globex overnight), paper Gateway, server 226; #1200.*

- **Order, `ext_operator`:** `TRADER7` and `12345` accepted on ES and echoed in OpenOrder.
  `probe-op` (hyphen) → 201 "Order rejected - reason:Invalid value in field # 8089", on AAPL
  and ES, alone or with `manual_order_indicator`.
- **Order, `manual_order_indicator`:** 0 and 1 accepted on AAPL (PreSubmitted). The OpenOrder
  echo reports it absent (`None`); only a rejected order echoed the value sent.
- **Cancel (`cancel_order_with`), ES Submitted orders:** default (control), `manual_order_indicator`
  0 or 1, and `ext_operator` `TRADER7` → OrderStatus Cancelled + 202 within ~1s.
  `ext_operator` `probe-cxl` (hyphen) → **no reply**: the order goes PendingCancel and stays
  there. A later plain cancel draws 10148 "cannot be cancelled, state: PendingCancel"; a global
  cancel draws 161. Not cleared by the end of the session.
- **Global cancel (`global_cancel_with`):** `TRADER7` + indicator 1 → 202 for each working order.
  `probe-gcx` (hyphen) → the order went PendingCancel and wedged, as above.
- **Not probed:** which characters besides `-` are rejected, length limits, any indicator value
  other than 0/1, `ext_operator` on a stock order with a valid value, whether a wedged
  PendingCancel clears at the daily reset, and live (non-paper) accounts.
