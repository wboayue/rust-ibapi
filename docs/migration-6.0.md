# Migration Guide: 5.x to 6.0

This guide covers the breaking changes in `ibapi` 6.0, which is in development on `main`. Each
breaking change adds its section here in the PR that makes it. For 4.x → 5.0, see
[`migration-5.0.md`](migration-5.0.md).

Planned changes are tracked on the
[6.0 milestone](https://github.com/wboayue/rust-ibapi/milestone/11).

## Breaking changes

### 1. `ibapi::protocol` is no longer public

The module held `Features`, `ProtocolFeature`, `check_version`, `is_supported` and
`include_if_supported`, so a caller could check a server version before making a request.
That check was never needed:

- **53 `Features` constants were deprecated in 5.1** because they sit at or below server
  version 213. The client refuses older servers at connect, so a check against any of them
  always passed. They're removed.
- **The three remaining gates (`CANCEL_CONTRACT_DATA`, `CONFIG`, `UPDATE_CONFIG`) are checked
  by the requests themselves.** A server too old for one returns
  `Error::ServerVersion(required, actual, feature)` from that call.
- **`is_supported` and `include_if_supported` are removed.** Nothing in the crate called them.

```rust,ignore
// 5.x
use ibapi::protocol::{check_version, Features};
check_version(client.server_version(), Features::POSITIONS)?;    // always Ok
check_version(client.server_version(), Features::CONFIG)?;
let config = client.config()?;

// 6.0: drop the check; the request reports an old server itself
match client.config() {
    Ok(config) => { /* ... */ }
    Err(Error::ServerVersion(required, actual, feature)) => {
        eprintln!("{feature} needs server {required}, connected to {actual}");
    }
    Err(e) => return Err(e),
}
```

To branch on the version without making the request, compare `client.server_version()` with
the number from the error or the TWS API docs (`CONFIG` is 219).

### 2. `ComboLegOpenClose` and `MarketDataType` keep unrecognized codes

Both mapped any code they did not model to a unit `Unknown`, losing the value. `ComboLegOpenClose::Unknown` was itself wire code `3`, so a leg TWS reported with code `4` or higher was sent back as `3` when the contract was reused in an order. Both now take the shape the order enums already use (`Liquidity`, `OcaType`, ...):

- **`Unknown` carries the code: `Unknown(i32)`.** `From<i32>` keeps any code outside the table, and the new `From<T> for i32` returns it unchanged. `ComboLegOpenClose` code `3` (IB's "unknown" position) is now `Unknown(3)`. An absent `MarketDataType` decodes as `Unknown(0)`, as before.
- **The variants no longer have explicit discriminants**, so `value as i32` no longer compiles. Use `i32::from(value)`.
- **`switch_market_data_type(MarketDataType::Unknown(n))` sends `n`.** It always sent `0` before.
- **`ToField for ComboLegOpenClose` is removed**: a leftover from the text wire format with no remaining caller.
- **Serde:** `ComboLegOpenClose::Unknown` serializes in the derived tagged form, `{"Unknown":3}`, where it was `"Unknown"`. The other variants are unchanged.

```rust,ignore
// 5.x
let code = leg.open_close as i32;
if market_data_type == MarketDataType::Unknown { /* ... */ }

// 6.0
let code = i32::from(leg.open_close);
if let MarketDataType::Unknown(code) = market_data_type { /* ... */ }
```

### 3. `From<i32> for ArticleType` is removed

It mapped any unknown code to `ArticleType::Text`, so a binary article type TWS added later would have read as text. `news_article` has not used it since 5.4.0: its decoder rejects an unknown code with `Error::Parse`. Nothing else in the crate called it. Match on the two variants directly; there is no replacement conversion.

### 4. `CommissionReport` is now `CommissionAndFeesReport`

IBKR renamed commissions to "commissions and fees" (C# `CommissionAndFeesReport`, proto
`CommissionAndFeesReport`), so the public names follow. The data and the wire are unchanged.
The renames also cover the what-if commission fields on `OrderState` and the message-type
variant.

| 5.x | 6.0 |
|---|---|
| `orders::CommissionReport` | `orders::CommissionAndFeesReport` |
| `CommissionReport::commission` | `CommissionAndFeesReport::commission_and_fees` |
| `PlaceOrder::CommissionReport(..)` | `PlaceOrder::CommissionAndFeesReport(..)` |
| `OrderUpdate::CommissionReport(..)` | `OrderUpdate::CommissionAndFeesReport(..)` |
| `Executions::CommissionReport(..)` | `Executions::CommissionAndFeesReport(..)` |
| `IncomingMessages::CommissionsReport` | `IncomingMessages::CommissionsAndFeesReport` |
| `OrderState::commission` | `OrderState::commission_and_fees` |
| `OrderState::minimum_commission` | `OrderState::min_commission_and_fees` |
| `OrderState::maximum_commission` | `OrderState::max_commission_and_fees` |
| `OrderState::commission_currency` | `OrderState::commission_and_fees_currency` |

```rust,ignore
// 5.x
Ok(OrderUpdate::CommissionReport(report)) => println!("{}", report.commission),
let cost = order_state.commission;

// 6.0
Ok(OrderUpdate::CommissionAndFeesReport(report)) => println!("{}", report.commission_and_fees),
let cost = order_state.commission_and_fees;
```

There's no deprecated `CommissionReport` alias. A type alias would keep only `use` lines
compiling, while every match arm and field read changes anyway. Pairing a report with its
execution by `execution_id` is unchanged.

### 5. `Contract::spread` takes the combo symbol

TWS rejects a combo without a symbol with error 321. In 5.x the symbol was an optional
`.symbol(..)` setter, so a combo built without it compiled, built, and failed only when the order
was placed. The symbol is now the argument to `Contract::spread`:

- **`Contract::spread(symbol)` and `SpreadBuilder::new(symbol)`** take it up front.
- **`SpreadBuilder::symbol(..)` and `impl Default for SpreadBuilder` are removed.**
- **`build()` returns `Error::InvalidArgument` for an empty symbol**, as it already does for a
  spread with no legs.

```rust,ignore
// 5.x
let spread = Contract::spread().symbol("AAPL").vertical(long_call, short_call).build()?;

// 6.0
let spread = Contract::spread("AAPL").vertical(long_call, short_call).build()?;
```

### 6. `TickType::Unknown` carries the tick id

`TickType` mapped any tick id it did not model to a unit `Unknown`, so a tick IBKR added later lost
its id on every market data stream. It now takes the shape of §2:

- **`Unknown(i32)`** holds the id as received. Match `TickType::Unknown(_)`, or bind the id.
- **The variants no longer have explicit discriminants**, so `tick_type as i32` no longer compiles.
  Use `i32::from(tick_type)`; `TickType::from(i32)` is unchanged for modeled ids.
- **`From<&str>` is replaced by `TickType::from_name(&str) -> Option<TickType>`**, which returns
  `None` for a name it doesn't know instead of `Unknown`. `name()` is its inverse. Both use the
  TWS API field names (`"bidSize"`, `"bidPrice"`), not the `Display` text.
- **`Display` of an unknown tick is `Unknown(<id>)`**, for example `Unknown(500)`, where it was
  `Unknown`.
- **`TickType::default()` is `Unknown(-1)`**, the same value as before.

```rust,ignore
// 5.x
let id = tick.tick_type as i32;
let bid_size = TickType::from("bidSize");
if tick.tick_type == TickType::Unknown { /* ... */ }

// 6.0
let id = i32::from(tick.tick_type);
let bid_size = TickType::from_name("bidSize"); // Option<TickType>
if let TickType::Unknown(id) = tick.tick_type { /* ... */ }
```

### 7. Fund enums keep unknown values; their fields are `Option`

`FundAssetType` and `FundDistributionPolicyIndicator` read any value they didn't recognize as
`None`, so a new code from TWS looked the same as no value. Now:

- **The `None` variant is removed** from both enums, along with their `Default`.
  `ContractDetails::fund_asset_type` and `fund_distribution_policy_indicator` are `Option<T>`:
  `None` when TWS sends no value.
- **An unrecognized value is `Unknown(String)`**, carrying the raw wire string.
- **`From<&str>` is replaced by `FromStr`** (`"004".parse::<FundAssetType>()`), which rejects
  only an empty string. `as_str()` and `Display` return the wire value (`"004"`, `"Y"`).
- **Serde:** `Unknown` serializes in the derived tagged form, `{"Unknown":"008"}`. An unset
  field is `null`, so JSON saved by 5.x with `"None"` for either field no longer deserializes.

```rust,ignore
// 5.x
if details.fund_asset_type == FundAssetType::Equity { /* ... */ }
if details.fund_asset_type == FundAssetType::None { /* no value */ }

// 6.0
if details.fund_asset_type == Some(FundAssetType::Equity) { /* ... */ }
if details.fund_asset_type.is_none() { /* no value */ }
```

### 8. `NewsArticle::time` is `Option`

An article with no time, or one the client couldn't parse, arrived dated 1970-01-01, which
looked like a real date to code that sorts or filters by time. A tick news timestamp out of
range instead ended the `contract_news` or `broad_tape_news` subscription with `Error::Parse`.
Now:

- **`NewsArticle::time` is `Option<OffsetDateTime>`**, `None` in all of those cases. The
  subscription carries on, and unparseable text or an out-of-range timestamp is logged with
  `log::warn!` and the raw value.
- **Serde:** an unset time serializes as `null`.

```rust,ignore
// 5.x
println!("{}", article.time);
articles.sort_by_key(|a| a.time);

// 6.0
match article.time {
    Some(time) => println!("{time}"),
    None => println!("time unknown"),
}
articles.sort_by_key(|a| a.time); // None sorts first
```

### 9. `MarketDepth` and `MarketDepthL2` `operation` and `side` are enums

`MarketDepth::operation`, `MarketDepth::side`, `MarketDepthL2::operation` and `MarketDepthL2::side` held the wire codes as `i32`: operation `0` insert, `1` update, `2` delete; side `0` ask, `1` bid. They are now `market_data::realtime::DepthOperation` (`Insert`, `Update`, `Delete`) and `market_data::realtime::DepthSide` (`Ask`, `Bid`), each with `Unknown(i32)` for a code this version does not model, which keeps the code as received. `DepthOperation::from(i32)`, `DepthSide::from(i32)` and `i32::from(..)` convert between the two.

```rust,ignore
// 5.x
if row.side == 1 {
    match row.operation {
        0 | 1 => bids[row.position as usize] = Some((row.price, row.size)),
        2 => bids[row.position as usize] = None,
        _ => {}
    }
}

// 6.0
use ibapi::market_data::realtime::{DepthOperation, DepthSide};

if row.side == DepthSide::Bid {
    match row.operation {
        DepthOperation::Insert | DepthOperation::Update => bids[row.position as usize] = Some((row.price, row.size)),
        DepthOperation::Delete => bids[row.position as usize] = None,
        DepthOperation::Unknown(_) => {}
    }
}
let code = i32::from(row.operation); // where the wire code is needed
```

Match on the variants, or call `i32::from(..)` where the wire code is needed. The compiler finds every comparison with an integer.

The compiler does not catch the serialized form. `MarketDepth` and `MarketDepthL2` derive `Serialize` / `Deserialize`, and `operation` and `side` now serialize in the enums' derived externally tagged form: `"Insert"`, `"Update"`, `"Delete"`, `"Ask"`, `"Bid"`, or `{"Unknown":9}`, where they were numbers. Stored or transmitted rows written by 5.x carry the numbers and no longer deserialize; the `utoipa` schemas change the same way.

### 10. `NewsBulletin::message_type` is a `BulletinType`

`NewsBulletin::message_type` held the bulletin's wire code as an `i32`: `1` regular bulletin, `2` exchange no longer available for trading, `3` exchange available for trading. It is now a `news::BulletinType` with a variant for each of those codes and `Unknown(i32)` for a code this version does not model, which keeps the code as received. `BulletinType::from(i32)` and `i32::from(BulletinType)` convert between the two.

```rust,ignore
// 5.x
match bulletin.message_type {
    2 => mark_closed(&bulletin.exchange),
    3 => mark_open(&bulletin.exchange),
    _ => {}
}

// 6.0
use ibapi::news::BulletinType;

match bulletin.message_type {
    BulletinType::ExchangeUnavailable => mark_closed(&bulletin.exchange),
    BulletinType::ExchangeAvailable => mark_open(&bulletin.exchange),
    BulletinType::Regular | BulletinType::Unknown(_) => {}
}
let code = i32::from(bulletin.message_type); // where the wire code is needed
```

Match on the variants, or call `i32::from(..)` where the wire code is needed. The compiler finds every comparison with an integer.

The compiler does not catch the serialized form. `NewsBulletin` derives `Serialize` / `Deserialize`, and `message_type` now serializes in the enum's derived externally tagged form: `"Regular"`, `"ExchangeUnavailable"`, `"ExchangeAvailable"`, or `{"Unknown":7}`, where it was the number. Stored or transmitted bulletins written by 5.x carry the number and no longer deserialize; the `utoipa` schema changes the same way.

### 11. `Missing` moves to `ibapi::builder`

`Missing`, the marker a typestate builder carries for a required field it has not been given, was `ibapi::contracts::Missing`. The order condition builders use it too now (§12), so it lives in a shared module, `ibapi::builder`. No other path is kept.

```rust,ignore
// 5.x
use ibapi::contracts::Missing;

// 6.0
use ibapi::builder::Missing;
```

Only code that names a builder's type needs the import, such as `OptionBuilder<Symbol, Missing, Missing>` in a signature.

### 12. Condition builders require a comparison before `build()`

`PriceConditionBuilder`, `TimeConditionBuilder`, `MarginConditionBuilder`, `VolumeConditionBuilder` and `PercentChangeConditionBuilder` panicked in `build()` when neither `greater_than()` nor `less_than()` had been called. They are now typestate builders: `build()`, and the `From<..> for OrderCondition` conversion that `OrderBuilder::condition`, `and_condition` and `or_condition` use, exist only after a comparison. A missing comparison is a compile error rather than a panic.

Code that sets a comparison before building compiles unchanged, through the builders and through the helpers in `orders::builder` (`price()`, `time()`, `margin()`, `volume()`, `percent_change()`):

```rust,ignore
let condition = price(265598, "SMART").greater_than(150.0).build(); // 5.x and 6.0
let condition = price(265598, "SMART").build();                     // 5.x: panicked; 6.0: does not compile
```

Each builder now takes a type parameter for its threshold, `Missing` until a comparison sets it (§11): `PriceConditionBuilder<f64>`, `TimeConditionBuilder<String>`, `MarginConditionBuilder<i32>`, `VolumeConditionBuilder<i32>`, `PercentChangeConditionBuilder<f64>`. Code that names a builder's type after the comparison writes it with the parameter; the bare name means the state before one.

`greater_than()` and `less_than()` can be called once per builder. A second call, which used to replace the first, no longer compiles.

### 13. `OptionBuilder::build()` returns `Result`

`Contract::call(..)` / `Contract::put(..)` checked the strike in `strike()` and panicked on one that was not positive and finite (zero, negative, NaN or infinite). `strike()` now takes any `f64`, and `build()` checks it, returning `Result<Contract, Error>`: `Error::InvalidArgument` for a bad strike. This is the shape of `SpreadBuilder::build()` and `ContractBuilder::build()`.

```rust,ignore
// 5.x
let option = Contract::call("AAPL").strike(150.0).expires_on(2024, 12, 20).build();

// 6.0
let option = Contract::call("AAPL").strike(150.0).expires_on(2024, 12, 20).build()?;
```

Every option `build()` call gains `?` (or handling of the error); the compiler finds each one where a `Contract` is expected.

After `strike()`, the builder's strike type parameter is `f64`, where it was the validated `Strike`. Code that names the type writes `OptionBuilder<Symbol, f64, ExpirationDate>`. `Strike` and `Strike::new` are unchanged.

### 14. Bool fields keep the protobuf `is_` prefix

Public bool fields take their protobuf field's name. Three dropped its `is_` prefix and now keep it:

| 5.x | 6.0 |
|---|---|
| `Execution::liquidation: i32` | `Execution::is_liquidation: bool` |
| `Execution::pending_price_revision` | `Execution::is_price_revision_pending` |
| `MarketDepthL2::smart_depth` | `MarketDepthL2::is_smart_depth` |

`liquidation` also changes type. It was an `i32` that was only ever `0` or `1`, a holdover from the text wire format; TWS sends a bool. An absent value is `false`, as `0` was before.

```rust,ignore
// 5.x
if execution.liquidation != 0 || execution.pending_price_revision { /* ... */ }
if row.smart_depth { /* ... */ }

// 6.0
if execution.is_liquidation || execution.is_price_revision_pending { /* ... */ }
if row.is_smart_depth { /* ... */ }
```

The compiler finds every use. It does not catch the serialized form: `MarketDepthL2` derives `Serialize` / `Deserialize`, so its JSON key is now `is_smart_depth`, and rows written by 5.x no longer deserialize. `Execution` has no serde derive. The `utoipa` schemas rename all three properties, and `is_liquidation` becomes a boolean.

### 15. PnL figures are `Option<f64>`

`PnLSingle::daily_pnl`, `unrealized_pnl`, `realized_pnl` and `value`, and `PnL::daily_pnl`, were `f64`. A figure TWS left out of the frame decoded as `0.0`, indistinguishable from a real zero. TWS leaves out `unrealized_pnl` for a flat position, for one. They are now `Option<f64>`, `None` for a figure TWS left out, as `PnL::unrealized_pnl` and `realized_pnl` already were.

```rust,ignore
// 5.x
println!("daily {:.2}", pnl.daily_pnl);

// 6.0
if let Some(daily) = pnl.daily_pnl {
    println!("daily {daily:.2}");
}
let daily = pnl.daily_pnl.unwrap_or(0.0); // where the old numeric reading is wanted
```

Handle `None`, or call `unwrap_or(0.0)` where the old reading of an unset figure as `0.0` is wanted. The compiler finds every use of the five fields as `f64`.

The compiler does not catch the serialized form. `PnL` and `PnLSingle` derive `Serialize` / `Deserialize`, and an unset figure now serializes as `null`, where it was a number; the `utoipa` schemas mark the five fields nullable. JSON written by 5.x still deserializes, but an unset figure stored as `0` reads back as `Some(0.0)`, not `None`.

### 16. `account_updates_multi` takes a `LedgerOnly`

`Client::account_updates_multi` (async and blocking) takes a third argument, `ledger_only: accounts::LedgerOnly`, TWS's `ledgerAndNLV` flag for the account-updates-by-model request. `LedgerOnly::Yes` delivers only the per-currency `$LEDGER-*` keys (`$LEDGER-CashBalance`, `$LEDGER-NetLiquidationByCurrency`, ...), TWS's light-weight request; `LedgerOnly::No` delivers those plus the account-level keys without the prefix (`NetLiquidation`, `BuyingPower`, `AvailableFunds`, ...). 5.x always sent `Yes`, so the account-level keys could not be requested.

```rust,ignore
use ibapi::accounts::LedgerOnly;

// 5.x
let updates = client.account_updates_multi(Some(&account), None)?;

// 6.0: `LedgerOnly::Yes` keeps the 5.x request
let updates = client.account_updates_multi(Some(&account), None, LedgerOnly::Yes)?;
// or ask for the account-level keys as well
let updates = client.account_updates_multi(Some(&account), None, LedgerOnly::No)?;
```

Add the argument. The compiler finds every call that lacks it. It does not choose the value for you, and `LedgerOnly` has no default: pass `Yes` to receive what 5.x received, or `No` to receive the account-level keys as well, as verified against a live gateway (`docs/wire-facts/accounts.md`).

### 17. `Order::fa_profile`, `basis_points` and `basis_points_type` are removed

All three are text-protocol fields: `faProfile` went out in the text `placeOrder` until IB desupported FA profiles at server 177, and `basisPoints` / `basisPointsType` came back in the text `openOrder` for EFP orders ("download only" in IB's client). IB's protobuf `Order` message, the only shape the crate has spoken since 3.0, has no field for any of them, so nothing has sent or read them since. They are removed. Deleting an assignment changes nothing TWS sees. For a Financial Advisor allocation, use `fa_group` and `fa_method`, which are transmitted.

```rust,ignore
// 5.x
let order = Order {
    fa_group: "Group1".to_owned(),
    fa_profile: "Profile1".to_owned(),    // no longer sent
    ..Order::default()
};

// 6.0
let order = Order {
    fa_group: "Group1".to_owned(),
    ..Order::default()
};
```

The compiler finds every use of the three fields. It does not catch the serialized form: a serialized `Order` no longer has the `fa_profile`, `basis_points` and `basis_points_type` keys. `Order` does not use `deny_unknown_fields`, so an `Order` serialized by 5.x still deserializes and the three keys are ignored.

### 18. `Error::Validation` replaces `Error::InvalidArgument` for builder failures

`OrderBuilder::submit`, `analyze` and `build_order`, the bracket and attached-order submits and `ExerciseOptionsBuilder::submit` check their inputs before sending anything. When that check failed they returned `Error::InvalidArgument` with the message text. They now return `Error::Validation(ValidationError)`, which carries the `ibapi::orders::builder::ValidationError` that `OrderBuilder::build` already returned: `InvalidQuantity`, `InvalidPrice`, `MissingRequiredField`, `InvalidCombination`, `InvalidStopPrice`, `InvalidLimitPrice`, `InvalidBracketOrder` or `InvalidPercentage`. `?` on a `ValidationError` in a function returning `Result<_, ibapi::Error>` gives the same variant.

```rust,ignore
// 5.x
match client.order(&contract).buy(0).market().submit() {
    Ok(order_id) => println!("placed {order_id}"),
    Err(Error::InvalidArgument(msg)) if msg.starts_with("Invalid quantity") => fix_quantity(),
    Err(e) => return Err(e),
}

// 6.0
use ibapi::orders::builder::ValidationError;

match client.order(&contract).buy(0).market().submit() {
    Ok(order_id) => println!("placed {order_id}"),
    Err(Error::Validation(ValidationError::InvalidQuantity(_))) => fix_quantity(),
    Err(e) => return Err(e),
}
```

Match `Error::Validation` and its `ValidationError` variant instead of `Error::InvalidArgument` and the message text.

The compiler does not catch any of this. `Error` is `#[non_exhaustive]`, so every match on it already has a wildcard arm: the old `Error::InvalidArgument` arm still compiles, no longer fires for these methods, and their failures fall through to the wildcard. The `Display` text changes from `InvalidArgument: Invalid quantity: 0` to `validation error: Invalid quantity: 0`, which matters to code that compares or parses it. `std::error::Error::source()` returns the `ValidationError`, where it returned `None`.

### 19. `cancel_and_drain` returns a failed cancel write as `Err`

`Subscription::cancel_and_drain` (sync and async) logged a failed cancel write and went on draining, so the failure showed up later, if at all, as `Drained::Unconfirmed` or a connection reset. It now returns the write's error. The cancel is also written when the stream had already ended with an error on our side (a decode error, `Error::BufferLimitExceeded`), where 5.x left it to the drop and its log. A `Drained::Unconfirmed` arm that stood for "the write may have failed" now gets those cases as `Err`; treat the request id as possibly live there too.

In async, the deadline now bounds the write as well. A write still pending when it passes gives `Drained::Unconfirmed` and finishes in its own task, where a failure is only logged. `cancel()` still returns `()`.

### 20. `Order::use_price_mgmt_algo` and `route_marketable_to_bbo` are `Option<bool>`

Both fields were `bool`. The order sent `true` as `1` and `false` as nothing, which leaves the TWS default in place, so a caller could not turn either feature off; an open order TWS reported with `0` (explicitly off) read as `false`, the same as unset. They are now `Option<bool>`: `None` sends nothing, `Some(false)` sends `0`, `Some(true)` sends `1`. An open order decodes absent / `0` / `1` the same way; TWS (server 226) sends both fields on every open order, so a read is `Some(false)` or `Some(true)`, and an order placed with `None` reads back `Some(false)` when the default is off.

```rust,ignore
// 5.x
order.use_price_mgmt_algo = true;
order.route_marketable_to_bbo = false; // could only mean "leave the TWS default"

// 6.0
order.use_price_mgmt_algo = Some(true);
order.route_marketable_to_bbo = None; // leave the TWS default
order.route_marketable_to_bbo = Some(false); // turn it off
```

Replace `true` with `Some(true)` and `false` with `None`, and use `Some(false)` where the feature must be off. The compiler finds every use of the two fields as `bool`.

The compiler does not catch two things. `route_marketable_to_bbo: Some(false)` is sent, so it needs server version 217 (`ADDITIONAL_ORDER_PARAMS_2`), as `Some(true)` always did; against an older server the order now fails with `Error::ServerVersion` where `false` used to pass. `Order` derives `Serialize` / `Deserialize`, and an unset field now serializes as `null`; JSON written by 5.x, which has `true` or `false`, still deserializes, as `Some(true)` or `Some(false)`, so a stored `false` that meant "leave the default" now sends `0`.

### 21. `historical_news` takes `total_results: u16`

`Client::historical_news` (async and blocking) took `total_results: u8`, which capped a request at 255 headlines. IB documents the maximum as 300. The parameter is now `u16`.

```rust,ignore
// 5.x
let total_results: u8 = 255;
let headlines = client.historical_news(contract_id, &["BRFG"], start, end, total_results)?;

// 6.0
let total_results: u16 = 300;
let headlines = client.historical_news(contract_id, &["BRFG"], start, end, total_results)?;
```

Integer literals and untyped bindings compile unchanged. Where the argument is a typed `u8`, the compiler reports it; convert with `.into()` or declare it `u16`. The compiler does not check the range: the crate sends any `u16` as given. TWS returns at most 300 headlines for a larger value, without a notice, and rejects 0 with error 321 ([wire facts](wire-facts/news.md#historical-news-total_results-range)).
