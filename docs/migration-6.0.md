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
- **Serde:** both serialize as the wire code; see §47.

```rust,ignore
// 5.x
let code = leg.open_close as i32;
if market_data_type == MarketDataType::Unknown { /* ... */ }

// 6.0
let code = i32::from(leg.open_close);
if let MarketDataType::Unknown(code) = market_data_type { /* ... */ }
```

### 3. `ArticleType` keeps unrecognized codes

In 5.x, `From<i32> for ArticleType` read any code it did not model as `Text`. `news_article`
did not use it; since 5.4.0 it has returned `Error::Parse` for such a code and dropped the
article. `ArticleType` now takes the shape of the other integer-coded enums (§2):

- **A new `Unknown(i32)` variant.** `news_article` returns the article with
  `ArticleType::Unknown(code)` and the body passed through unchanged; it no longer fails.
  `From<i32>` returns `Unknown(code)` outside `0` (`Text`) and `1` (`Binary`). The new
  `From<ArticleType> for i32` returns the code unchanged.
- **The variants no longer have explicit discriminants**, so `value as i32` no longer compiles.
  Use `i32::from(value)`.
- An exhaustive `match` needs an arm for `Unknown`. `ArticleType` is now also `Copy` and `Eq`.

```rust,ignore
// 6.0
match article.article_type {
    ArticleType::Text => { /* HTML or plain text */ }
    ArticleType::Binary => { /* Base64; decode it */ }
    ArticleType::Unknown(code) => { /* a type this version doesn't model */ }
}
```

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
- **Serde:** both serialize as the wire string, `Unknown` included (§47). An unset field is
  `null`, so JSON saved by 5.x with `"None"` for either field no longer deserializes.

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

Serialized, `operation` and `side` are the wire code (§47), the number 5.x wrote, so `MarketDepth` and `MarketDepthL2` JSON written by 5.x still deserializes. The `utoipa` schemas are integers, as before.

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

Serialized, `message_type` is the wire code (§47), the number 5.x wrote, so `NewsBulletin` JSON written by 5.x still deserializes. The `utoipa` schema is an integer, as before.

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

`Client::account_updates_multi` (async and blocking) takes a third argument, `ledger_only: accounts::LedgerOnly`, TWS's `ledgerAndNLV` flag for the account-updates-by-model request. `LedgerOnly::Yes` delivers only the per-currency ledger keys (`CashBalance`, `NetLiquidationByCurrency`, ...), TWS's light-weight request; `LedgerOnly::No` delivers those plus the account-level keys (`NetLiquidation`, `BuyingPower`, `AvailableFunds`, ...). The ledger keys carry a `$LEDGER-` prefix (`$LEDGER-CashBalance`) only when the TWS/Gateway API setting "Prepend '$LEDGER-' prefix to per-currency account values" is on. 5.x always sent `Yes`, so the account-level keys could not be requested.

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

// 6.0 (with §26's builder)
let total_results: u16 = 300;
let headlines = client.historical_news(contract_id, &["BRFG"]).since(start).limit(total_results).subscribe()?;
```

Integer literals and untyped bindings compile unchanged. Where the argument is a typed `u8`, the compiler reports it; convert with `.into()` or declare it `u16`. The compiler does not check the range: the crate sends any `u16` as given. TWS returns at most 300 headlines for a larger value, without a notice, and rejects 0 with error 321 ([wire facts](wire-facts/news.md#historical-news-total_results-range)).

§26 replaces the signature with a builder; `total_results` is now `.limit(n)`.

### 22. `historical_data(..).stream()` is `subscribe_with_updates()`, yielding `HistoricalDataFrame`

The keep-up-to-date terminal is renamed `subscribe_with_updates()`, beside `subscribe()`, and both yield one enum, `HistoricalDataFrame`. `HistoricalBarUpdate` is gone:

| 5.x `HistoricalBarUpdate` | 6.0 `HistoricalDataFrame` |
|---|---|
| `Historical(HistoricalData)` | `Bars(Vec<Bar>)` |
| `Update(Bar)` | `Update(Bar)` |
| `End { start, end }` | `End { start, end }` |

`Historical` held a `HistoricalData` whose `start` and `end` were always `OffsetDateTime::UNIX_EPOCH` placeholders: TWS sends the requested window in a separate frame, which the stream already yields as `End { start, end }`. `Bars` holds only the bars. `historical_data(..).fetch()` and `HistoricalData` are unchanged.

```rust,ignore
// 5.x
let subscription = client.historical_data(&contract, BarSize::Min).duration(1.days()).stream()?;
for update in subscription.iter_data() {
    match update? {
        HistoricalBarUpdate::Historical(data) => println!("{} bars", data.bars.len()),
        HistoricalBarUpdate::Update(bar) => println!("{bar:?}"),
        HistoricalBarUpdate::End { start, end } => println!("{start} - {end}"),
    }
}

// 6.0
let subscription = client.historical_data(&contract, BarSize::Min).duration(1.days()).subscribe_with_updates()?;
for update in subscription.iter_data() {
    match update? {
        HistoricalDataFrame::Bars(bars) => println!("{} bars", bars.len()),
        HistoricalDataFrame::Update(bar) => println!("{bar:?}"),
        HistoricalDataFrame::End { start, end } => println!("{start} - {end}"),
    }
}
```

Rename the terminal and the type, bind the `Vec<Bar>` in the `Bars` arm, and take the window from `End`. The compiler finds all of these. It does not catch the serialized form: `HistoricalDataFrame` derives `Serialize` / `Deserialize` as `HistoricalBarUpdate` did, and the first frame is now `{"Bars":[..]}` where it was `{"Historical":{start, end, bars}}`, so JSON written by 5.x does not deserialize. The `utoipa` schema is renamed and updated the same way.

`subscribe()`, new in 6.0, never yields `Update`, but a `match` on its frames needs an arm for it. To trim with `HistoricalData::trim_to`, build `HistoricalData { start, end, bars }` from the `Bars` and `End` frames, as the `historical_data_subscribe` examples do.

### 23. `TickTypes` and `MarketDepths` gain `Reroute`

TWS answers a market data or market depth request for a CFD with the underlying contract to request in its place. Those frames reached no subscription and were dropped with an info-level log. A `market_data` subscription now yields them as `TickTypes::Reroute(Reroute)` and a `market_depth` subscription as `MarketDepths::Reroute(Reroute)`, where `market_data::realtime::Reroute` carries the underlying's `contract_id` and `exchange`. It is the last item: TWS has already ended the CFD request, so the stream ends after it and drop writes no cancel. Stock CFDs are rerouted to the stock; index CFDs are not. `Quote::apply` ignores `TickTypes::Reroute`.

```rust,ignore
// 5.x
match depth? {
    MarketDepths::MarketDepth(row) => apply(&mut book, row),
    MarketDepths::MarketDepthL2(row) => apply_l2(&mut book, row),
    MarketDepths::Reset => book.clear(),
}

// 6.0
match depth? {
    MarketDepths::MarketDepth(row) => apply(&mut book, row),
    MarketDepths::MarketDepthL2(row) => apply_l2(&mut book, row),
    MarketDepths::Reset => book.clear(),
    MarketDepths::Reroute(reroute) => resubscribe(reroute.contract_id, &reroute.exchange),
}
```

Add a `Reroute` arm to exhaustive matches on `TickTypes` and `MarketDepths`; on it, request the named contract on the named exchange. The compiler finds every exhaustive match, since neither enum is `#[non_exhaustive]`. It does not catch a match with a wildcard arm (`_ => {}`): that compiles unchanged and drops the reroute as silently as 5.x did. `MarketDepths` derives `Serialize` / `Deserialize`, and the new variant serializes as `{"Reroute":{"contract_id":..,"exchange":".."}}`, which a 5.x reader does not deserialize.

### 24. `ContractDetails::market_rule_ids` is `Vec<i32>`

`ContractDetails::market_rule_ids` held TWS's comma-separated market rule ids as strings, although `Client::market_rule` takes an `i32`, so each id had to be parsed first. It is now `Vec<i32>`. An absent or empty list is still empty. A `contract_details` reply whose list has an entry that is not an integer now fails with `Error::Parse`, where 5.x kept the text.

```rust,ignore
// 5.x
let rule_id: i32 = details.market_rule_ids.first().and_then(|id| id.parse().ok()).expect("no market rule ids");
let rule = client.market_rule(rule_id)?;

// 6.0
let rule_id = details.market_rule_id("SMART").expect("no market rule for SMART");
let rule = client.market_rule(rule_id)?;
```

Drop the `.parse()` and pass the id directly. The ids are one per entry of `valid_exchanges`, in the same order, and can differ by exchange, so `.first()` is the first valid exchange's rule, not necessarily the one you trade on. `ContractDetails::market_rule_id(exchange)`, new in 6.0, looks it up. The compiler finds every use that treats an id as a string. It does not catch two runtime differences: a non-integer id in a reply is now an `Error::Parse` from `contract_details`, and `ContractDetails` derives `Serialize` / `Deserialize`, so the ids now serialize as numbers and JSON written by 5.x, with string ids, no longer deserializes. The `utoipa` schema is an integer array.

### 25. `Client::notice_stream()` returns `NoticeStream`, not `Result`

`Client::notice_stream()` (async and blocking) returned `Result<NoticeStream, Error>`, but it never returned `Err`: subscribing to the notice stream cannot fail. It now returns the `NoticeStream` directly. `ClientBuilder::connect_with_notice_stream()` is unchanged.

```rust,ignore
// 5.x
let notices = client.notice_stream()?;
let notices = client.notice_stream().expect("notice subscription failed");

// 6.0
let notices = client.notice_stream();
```

Drop the `?`, `.expect(..)` or `match` on the result. The compiler reports each of them. No error path is lost, since none existed, and nothing changes at runtime.

### 26. `historical_news` returns a builder

`Client::historical_news` (async and blocking) took `start_time`, `end_time` and
`total_results` and returned the headline stream. It now takes `contract_id` and
`provider_codes` and returns a `HistoricalNewsBuilder`:

- `.before(time)`: headlines strictly before `time`.
- `.since(time)`: headlines at or after `time`.
- `.limit(n)`: at most `n` headlines, 1 – 300; default 300.
- `.subscribe()`: the stream `historical_news` returned.
- `.page()`: a `HistoricalNewsPage`, the headlines plus TWS's `has_more` flag.

```rust,ignore
// 5.x
let headlines = client.historical_news(contract_id, &["BRFG"], start, end, 10)?;

// 6.0
let headlines = client.historical_news(contract_id, &["BRFG"]).since(start).limit(10).subscribe()?;
```

**Results change.** The 5.x range had no effect: the times were sent in a format TWS ignores
without a notice, so every request returned the newest headlines. TWS also uses only one bound per
request: given both, it reads the start time as the newest bound and ignores the end time
([wire facts](wire-facts/news.md#historical-news-range-and-has_more)). So pick the bound that
matches the intent. For "what happened since `start`", use `.since(start)`. For "the latest
headlines before `end`", use `.before(end)`.
A 5.x call whose range was not the latest headlines returned the wrong ones; the same call with a
bound now returns what it asked for.

The compiler reports every call. Nothing it can check depends on which bound you choose.

### 27. Id parameters take `impl Into<ContractId>` and `impl Into<AccountId>`

These id parameters now accept both the raw value and the typed id:

| Method (sync and async) | 5.x | 6.0 |
|---|---|---|
| `option_chain`, `wsh_event_data_by_contract`, `historical_news` | `contract_id: i32` | `contract_id: impl Into<ContractId>` |
| `OrderBuilder::account`, `ExerciseOptionsBuilder::account` | `impl Into<String>` | `impl Into<AccountId>` |
| `pnl`, `pnl_single`, `account_updates` | `account: &AccountId` | `account: impl Into<AccountId>` |
| `pnl_single` | `contract_id: ContractId` | `contract_id: impl Into<ContractId>` |

The crate hands back raw ids (`managed_accounts()` returns `String`s, `Contract::contract_id` is an `i32`), yet `pnl*` and `account_updates` accepted only the typed ids, while the other inputs accepted only raw ones. Now either passes to all of them. `positions_multi` and `account_updates_multi` keep `Option<&AccountId>`. `AccountId` gains `From<&String>` and `From<&AccountId>`. What goes on the wire is unchanged.

Integer literals, `i32`, `ContractId`, `&str`, `String`, `&String`, `AccountId` and `&AccountId` compile as before. Two kinds of argument stop compiling:

- **An argument whose type came from the parameter.** `.into()`, `.try_into()` or `.parse()` passed straight to an id parameter has no target type any more; name it.
- **A value that converts only into `String`.** `Cow<str>`, `Box<str>`, `char`, `&mut str`, a user type with only `Into<String>`, or a generic `impl Into<String>` forwarded to `account(..)`: convert to `String` first.

```rust,ignore
let account: String = client.managed_accounts()?.remove(0);
let field: &str = "265598";
let cow: Cow<str> = Cow::Borrowed("DU1234567");

// 5.x
let pnl = client.pnl(&AccountId(account.clone()), None)?;
let single = client.pnl_single(&AccountId(account.clone()), ContractId(contract.contract_id), None)?;
let chain = client.option_chain("AAPL", SecurityType::Stock, field.parse()?);
let order = client.order(&contract).buy(100).market().account(cow);

// 6.0: the 5.x pnl / pnl_single lines still compile; these are the shorter forms
let pnl = client.pnl(&account, None)?;
let single = client.pnl_single(&account, contract.contract_id, None)?;
let chain = client.option_chain("AAPL", SecurityType::Stock, field.parse::<i32>()?); // was inferred from i32
let order = client.order(&contract).buy(100).market().account(cow.into_owned());     // Cow<str> isn't Into<AccountId>
```

The compiler reports every call that no longer compiles. Nothing changes at runtime.

### 28. `last_trade_date_or_contract_month` holds only the date

`contract_details` decoded a future's or option's `last_trade_date_or_contract_month` as TWS sends it, with the last trade time and zone appended (`"20261218 08:30:00 US/Central"`). It now holds the date alone (`"20261218"`), the time goes to `ContractDetails::last_trade_time` (`"08:30:00"`), and the zone is dropped; `ContractDetails::time_zone_id` holds the product's zone. This is what IB's own clients do (`EDecoderUtils.SetLastTradeDate`). Bonds are unchanged.

```rust,ignore
// 5.x
let date = details.contract.last_trade_date_or_contract_month.split_whitespace().next().unwrap_or_default();

// 6.0
let date = &details.contract.last_trade_date_or_contract_month; // Option<Expiry>, see §30
let time = &details.last_trade_time;
let zone = &details.time_zone_id;
```

Code that split the field itself can drop that step. The field's type changes too (§30). For a typed date, `details.contract.last_trade_date` (`Option<time::Date>`) already carries the last trading day of a future or option.

### 29. `exercise_options` yields `PlaceOrder`; `ExerciseOptions` is removed

TWS reports an exercise as an order on the option, so an `exercise_options` subscription now yields `PlaceOrder`, the item type of `place_order`, and `ExerciseOptions` is gone. The subscription also yields the exercise's two executions (on the underlying and on the option) as `PlaceOrder::ExecutionData`, each with `Execution::option_exercise_type` set; they were dropped before. TWS sends no commission report for them.

```rust,ignore
// 5.x
match item {
    SubscriptionItem::Data(ExerciseOptions::OpenOrder(order)) => {}
    SubscriptionItem::Data(ExerciseOptions::OrderStatus(status)) => {}
    SubscriptionItem::Notice(notice) => {}
}

// 6.0
match item {
    SubscriptionItem::Data(PlaceOrder::OpenOrder(order)) => {}
    SubscriptionItem::Data(PlaceOrder::OrderStatus(status)) => {}
    SubscriptionItem::Data(PlaceOrder::ExecutionData(data)) => {
        // data.execution.option_exercise_type is Exercise, Lapse, ...
    }
    SubscriptionItem::Data(PlaceOrder::CommissionAndFeesReport(_)) => {}
    SubscriptionItem::Notice(notice) => {}
}
```

Rename `ExerciseOptions` to `PlaceOrder` and add the two arms; the compiler reports each site. `ExerciseOptions` derived `PartialEq` and serde; `PlaceOrder` does not, so code that compared or serialized the items compares or serializes the fields it needs instead. The subscription does not end on its own: stop reading after the execution on the option (the `ExecutionData` whose `contract.security_type` is the option's). An open order and a `Filled` status follow it and repeat the fill; the earlier `Filled` status is no end marker, since its `filled` counts the underlying's shares (100 for one contract).

### 30. `Contract::last_trade_date_or_contract_month` is `Option<Expiry>`

The field was a `String` that held `YYYYMMDD` (last trading day) or `YYYYMM` (contract month), and a typo surfaced only as TWS error 200. It is now `Option<Expiry>`:

- `Expiry::Date(ExpirationDate)` and `Expiry::Month(ContractMonth)` for the two forms.
- `Expiry::Unknown(String)` for any other text TWS sends: a bond's `"20430504 16:00 Europe/London"`, or a shape not seen yet. It is sent back to TWS as received. Parsing never produces it; decoding and deserializing do, so a contract in a position or order stream never fails to decode over this field.
- `None` where the field was `""`: stocks, an open futures query.

`ExpirationDate` and `ContractMonth` gain `FromStr` and `TryFrom<&str>`, which check that the text is a real date or month. `Contract::option` takes an `ExpirationDate` instead of `&str`: it returns a `Contract`, not a `Result`, so it had no way to report a bad string.

```rust,ignore
// 5.x
let contract = Contract { last_trade_date_or_contract_month: "202612".into(), ..Default::default() };
let call = Contract::option("AAPL", "20261218", 150.0, OptionRight::Call);
if details.contract.last_trade_date_or_contract_month.is_empty() { /* none */ }
println!("{}", details.contract.last_trade_date_or_contract_month);

// 6.0
let contract = Contract { last_trade_date_or_contract_month: Some(ContractMonth::new(2026, 12)?.into()), ..Default::default() };
let call = Contract::option("AAPL", ExpirationDate::new(2026, 12, 18)?, 150.0, OptionRight::Call); // §34
if details.contract.last_trade_date_or_contract_month.is_none() { /* none */ }
if let Some(expiry) = &details.contract.last_trade_date_or_contract_month {
    println!("{expiry}"); // Display is the wire text: "20261218"
}
match details.contract.last_trade_date_or_contract_month {
    Some(Expiry::Date(date)) => {}
    Some(Expiry::Month(month)) => {}
    Some(Expiry::Unknown(raw)) => {}
    None => {}
}
```

- `ContractBuilder::last_trade_date_or_contract_month("202612")` still takes a string. `build()` now rejects one that is neither form with `Error::InvalidArgument`, including the zoned `"20261218 08:30:00 US/Central"` form TWS also accepts (send the date alone). `ContractBuilder::expiry(..)` takes a typed value.
- `""` passed to the string setter still means unset, as in 5.x.
- `Expiry` is not ordered: a month and a date don't compare. To sort or filter contracts by expiry, use `contract.last_trade_date` (`Option<time::Date>`), or compare `ExpirationDate` / `ContractMonth`, which are `Ord` and `Hash`.
- A contract sent as a month comes back from TWS as its last trading day: an order on ES `202612` is echoed as `20261218`. Match on the variant you get, not the one you sent.
- Serialized, the field is the wire string as before (`"20261218"`); an unset field is `null` instead of `""`. Deserializing still reads `""` as `None`, so 5.x JSON loads. Any other text loads as `Expiry::Unknown`, unchecked.
- `Expiry` is in the prelude. The compiler reports every site that reads or writes the field.

### 31. `cancel_order` takes only the order id

`Client::cancel_order(order_id, manual_order_cancel_time)` (async and blocking) took the manual cancel time as a `&str`. It now takes only the order id and sends an immediate cancel with default `OrderCancel` fields (no manual cancel time, operator ID or manual order indicator), as `cancel_order(id, "")` did. `cancel_order_with(order_id, &OrderCancel)`, new in 6.0, carries the time in `OrderCancel::manual_order_cancel_time`, beside the operator ID and manual order indicator. `global_cancel()` and `global_cancel_with(&OrderCancel)` are unchanged.

```rust,ignore
// 5.x
client.cancel_order(order_id, "")?;
client.cancel_order(order_id, "20260101-15:30:00")?;

// 6.0
use ibapi::orders::OrderCancel;

client.cancel_order(order_id)?;
client.cancel_order_with(
    order_id,
    &OrderCancel {
        manual_order_cancel_time: "20260101-15:30:00".to_owned(),
        ..OrderCancel::default()
    },
)?;
```

The compiler flags every call. `cancel_order(id)` sends the same request as 5.x's `cancel_order(id, "")`; nothing changes on the wire.

### 32. `Order`'s order-type fields are `Option<OrderType>`, not `String`

| Field | 5.x | 6.0 |
|---|---|---|
| `Order::order_type` | `String` (`"LMT"`, `""` unset) | `Option<OrderType>` (`None` unset) |
| `Order::adjusted_order_type` | `String` (`"TRAIL"`, `""` unset) | `Option<OrderType>` |
| `Order::delta_neutral_order_type` | `String` (`"LMT"`, `""` unset) | `Option<OrderType>` |

`Order::order_type` held the TWS order-type code as a `String` while `OrderBuilder::order_type` took the builder's `OrderType` enum, so an order read back from `open_orders()` could only be compared to what was built by its code string; `adjusted_order_type` and `delta_neutral_order_type` held codes from the same set. All three are now `Option<orders::OrderType>` (also at `orders::builder::OrderType`), the enum the builder takes. `Display` and `as_str()` give the code, `s.parse::<OrderType>()` reads one, and a code this version does not list decodes as `OrderType::Unknown(raw)` and is sent back unchanged, so an order read from TWS round-trips. `None` is unset: `Order::default()` holds it, it is sent as no value, as the empty string was, and an absent or empty wire value decodes to it. TWS reports an unset `adjusted_order_type` or `delta_neutral_order_type` as the string `"None"` (see `docs/wire-facts/orders.md`); that decodes to `None` as well, so neither field is ever `Some(Unknown("None"))` on an order read from TWS. `attach_adjustable_stop` writes `Some(OrderType::Stop)`, `Some(OrderType::StopLimit)` or `Some(OrderType::TrailingStop)` where it wrote `"STP"`, `"STP LMT"` or `"TRAIL"`.

Five variants that shared a code with another are removed: `ComboLimit` and `ComboMarket` sent `LMT` / `MKT`, the same as `Limit` / `Market`; `MarketOnOpen`, `LimitOnOpen` and `AtAuction` sent `MKT`, `LMT` and `MTL` and set `TimeInForce::OnOpen` or `Auction`. `.order_type(..)` no longer sets a time in force; `market_on_open()`, `limit_on_open(price)` and `at_auction(price)` set the type and the time in force, as before.

```rust,ignore
let open_order: OrderData = /* from open_orders() */;
let stop: Order = /* from attach_adjustable_stop(..) */;

// 5.x
let order = Order { order_type: "LMT".to_string(), ..Default::default() };
let is_limit = open_order.order.order_type == "LMT";
let code: &str = open_order.order.order_type.as_str();
let adjusts_to_trail = stop.adjusted_order_type == "TRAIL";
let on_open = Order::builder().buy(100).limit(50.0).order_type(OrderType::LimitOnOpen).build()?;

// 6.0
let order = Order { order_type: Some(OrderType::Limit), ..Default::default() };     // Some(..) in literals
let is_limit = open_order.order.order_type == Some(OrderType::Limit);               // compare to Some(..)
let code: &str = open_order.order.order_type.as_ref().map_or("", OrderType::as_str); // as_str() is on OrderType, not Option
let adjusts_to_trail = stop.adjusted_order_type == Some(OrderType::TrailingStop);
let on_open = Order::builder().buy(100).limit_on_open(50.0).build()?;             // LimitOnOpen is removed
```

Wrap the variant in `Some(..)` in struct literals and comparisons (`OrderType::as_str` lists the codes), replace `ComboLimit` / `ComboMarket` with `Limit` / `Market`, and `.order_type(MarketOnOpen | LimitOnOpen | AtAuction)` with `market_on_open()`, `limit_on_open(..)` or `at_auction(..)`. A code the enum does not list is sent as `Some(OrderType::Unknown("SNAP MID".into()))` or `"SNAP MID".parse::<OrderType>()?`; `.order_type(OrderType::Unknown(..))` sends every price set earlier in the chain, unchecked, since the builder can't know which the code takes. The compiler finds every struct literal, assignment and comparison that uses a `String` or `&str` or a bare `OrderType`, every `.as_str()` or `.is_empty()` on the three fields, every use of the five removed variants, and every exhaustive `match` on `OrderType`, which needs an `Unknown(_)` arm. It does not catch the serialized forms. `Order` derives `Serialize` / `Deserialize`: an unset field now serializes as `null` where it was `""` (or `"None"` for an adjusted or delta-neutral type read from TWS); JSON written by 5.x still deserializes, with `""` and `"None"` read as `None`. `OrderType` on its own was the variant name (`"Limit"`) and is now the code (`"LMT"`), so `OrderType` JSON written by 5.x deserializes as `Unknown("Limit")` rather than failing, and the `utoipa` schema becomes a plain string where it listed the variant names; the three `Order` fields are nullable strings. An order type TWS reports that this version does not list no longer reads as a string to compare against; match `Some(OrderType::Unknown(code))` instead. `Order::is_delta_neutral()` is `delta_neutral_order_type.is_some()`; TWS reports an order without a hedge with the string `"None"` in that field, which 5.x kept, so it returned `true` for every order read from TWS and now returns `false`. One builder change is also runtime only: `MarketToLimit` now sends a limit price when one is set, so `.limit(p).order_type(OrderType::MarketToLimit)` sends `p` as the limit where 5.x dropped it (TWS accepts it on a day or GTC order and drops it there, echoing 0); `market_to_limit()` clears an earlier limit price and `at_auction(p)` sends `p` whatever time in force follows, both as in 5.x.

### 33. `AccuDistrBuilder::route_order_type` takes an `OrderType`

The child-order type of an AccuDistr algo was a free-form string; it is now the `OrderType` that `Order::order_type` holds.

```rust
// 5.x
accu_distr().route_order_type("LMT")

// 6.0
use ibapi::orders::OrderType;
accu_distr().route_order_type(OrderType::Limit)
```

The wire value is unchanged (`OrderType::as_str` lists the codes). A code the enum does not list is `OrderType::Unknown("CODE".into())`. The compiler finds every call.

### 34. `ExpirationDate::new` and `ContractMonth::new` return `Result`

Both built any value they were given: `ExpirationDate::new(2026, 13, 40)` was sent to TWS as
`20261340` and surfaced only as TWS error 200. They now return `Result<Self, Error>`, with
`Error::InvalidArgument` unless the components form a real calendar date (`ExpirationDate`) or
month 1–12 (`ContractMonth`), with a year of at most 9999. `FromStr` and `TryFrom<&str>` already
checked text this way.

```rust,ignore
// 5.x
let date = ExpirationDate::new(2026, 12, 18);
let es = Contract::futures("ES").expires_in(ContractMonth::new(2026, 12)).build();

// 6.0
let date = ExpirationDate::new(2026, 12, 18)?;
let es = Contract::futures("ES").expires_in(ContractMonth::new(2026, 12)?).build();
```

`OptionBuilder::expires_on(year, month, day)` still chains: `build()`, which already returns
`Result` (§13), now also returns `Error::InvalidArgument` for a date that isn't real. The
compiler finds every `new` call; it does not find an `expires_on` date that used to pass and now
fails at `build()`. `ExpirationDate::next_friday()`, `third_friday_of_month()`,
`ContractMonth::front()` and `next_quarter()` are unchanged.

### 35. `ContractDetails` gains the event-contract fields

`ContractDetails` has three new `String` fields, `event_contract_topic`,
`event_contract_description` and `event_contract_question`, which TWS sends for ForecastEx event
contracts and leaves out for everything else (empty there). Code that reads fields, or builds a
`ContractDetails` with `..Default::default()`, is unaffected. Two forms stop compiling:

- A struct literal that names every field: add the three, or end it with
  `..Default::default()`.
- A destructuring pattern that names every field: add them, or end it with `..`.

Serialized `ContractDetails` JSON from 5.x still deserializes: the three fields default to
empty when absent.

### 36. `ExecutionFilter::specific_dates` is `Vec<time::Date>`

`specific_dates` was a `Vec<String>` of `yyyymmdd` text. An entry that did not parse as an
integer, such as `"2026-01-30"` or `"20260130 "`, was dropped without an error, so the filter
asked for fewer dates than the caller gave, and with none left the request had no date filter
(for an otherwise default filter, today only).
The field now holds `time::Date`s, so a malformed date can't be written.

```rust,ignore
// 5.x
let filter = ExecutionFilter { specific_dates: vec!["20260130".to_string()], ..Default::default() };

// 6.0
use time::macros::date;
let filter = ExecutionFilter { specific_dates: vec![date!(2026 - 01 - 30)], ..Default::default() };
```

The compiler finds every site that builds the list. A date read from text parses with
`time::Date::parse(text, time::macros::format_description!("[year][month][day]"))`. TWS reads
each date as the execution's day in UTC or in the operator's time zone (live tests did not
distinguish between the two), not in the instrument's; this is unchanged. IB's `ExecutionFilter.proto` asks for dates
within the past week; whether TWS returns executions for an older date is not known
([wire facts](wire-facts/orders.md#executions-last_n_days-and-specific_dates)).

### 37. `Order::adjustable_trailing_unit` is an `AdjustableTrailingUnit`

`adjustable_trailing_unit` was an `i32` whose rustdoc, following IB's `Order` reference, gave
`0` for an amount and `1` for a percentage. TWS rejects `1` with error 201 and takes `100` for
a percentage ([wire facts](wire-facts/orders.md#adjustable_trailing_unit)). The field is now an
`orders::AdjustableTrailingUnit`: `Amount` (`0`, the default) or `Percent` (`100`), with `Unknown(i32)`
for a code this version does not model.

```rust,ignore
// 5.x
order.adjustable_trailing_unit = 100;

// 6.0
order.adjustable_trailing_unit = AdjustableTrailingUnit::Percent;
```

`attach_adjustable_stop` sets the field from its `TrailBy` as before. `i32::from(unit)` gives the
code. The compiler finds every assignment and comparison that uses an integer. Serialized, the
field is the wire code (§47), the number 5.x wrote, so `Order` JSON written by 5.x still
deserializes it.

### 38. `Bar::volume`, `wap` and `count` are `Option`

On both bar types, realtime (`market_data::realtime::Bar`) and historical
(`market_data::historical::Bar`), `volume` and `wap` are `Option<f64>` and `count` is
`Option<i32>`. Each is `None` on a bar type without trade data: MIDPOINT, BID, ASK, BID_ASK and
the like. A TRADES bar in which nothing traded is `Some(0.0)` / `Some(0)`.

5.x had a different "not applicable" per field. A realtime bar decoded the volume and WAP TWS
leaves out as `0.0`, the same as a quiet TRADES bar, and kept `count` as the `-1` TWS sends.
A historical bar kept the `-1` TWS sends for all three, so summing `volume` over MIDPOINT
bars gave a negative total.

The rule goes by value, not by `WhatToShow`: volume and count are `None` when TWS leaves them
out or sends `-1`, and WAP is `None` whenever volume is. ADJUSTED_LAST and AGGTRADES bars carry
real trade values. HISTORICAL_VOLATILITY and OPTION_IMPLIED_VOLATILITY bars come through as
volume `Some(1.0)` and count `Some(0)`, neither a traded quantity.

```rust,ignore
// 5.x
println!("volume {} vwap {:.2} trades {}", bar.volume, bar.wap, bar.count);
if bar.count != -1 { /* TRADES bar */ }

// 6.0
if let (Some(volume), Some(wap), Some(count)) = (bar.volume, bar.wap, bar.count) {
    println!("volume {volume} vwap {wap:.2} trades {count}");
}
if bar.count.is_some() { /* bar with trade data */ }
let volume = bar.volume.unwrap_or(0.0); // where a plain number is wanted
```

The compiler finds every use as `f64` or `i32`. It doesn't catch the serialized form: both
types derive `Serialize` / `Deserialize`, and an absent value now serializes as `null`; the
`utoipa` schemas mark the three fields nullable. JSON written by 5.x still deserializes, but a
stored `-1` reads back as `Some(-1.0)` / `Some(-1)`, not `None`, and a realtime bar's stored
`0` volume as `Some(0.0)`. Re-fetch, or map `-1` to `None` after loading, where the
distinction matters.

### 39. Tick-by-tick and historical ticks share `TickLast`, `TickBidAsk` and `TickMidpoint`

A tick-by-tick frame carries the same three TWS tick messages as a historical ticks reply, but
5.x decoded them into two unrelated families of structs. Both requests now yield one set of
types, defined in `market_data` and re-exported from `market_data::realtime` and
`market_data::historical`. Field names follow the proto's (`time`, `price_bid`, `size_bid`);
each struct's attribute field is `attributes`.

| 5.x | 6.0 |
|---|---|
| `realtime::Trade` | `TickLast` |
| `realtime::BidAsk` | `TickBidAsk` |
| `realtime::MidPoint` | `TickMidpoint` |
| `realtime::TradeAttribute`, `realtime::BidAskAttribute` | `TickAttributeLast`, `TickAttributeBidAsk` |
| `Trade::trade_attribute`, `BidAsk::bid_ask_attribute` | `attributes` |
| `BidAsk::bid_price`, `ask_price`, `bid_size`, `ask_size` | `price_bid`, `price_ask`, `size_bid`, `size_ask` |
| `MidPoint::mid_point` | `price` |
| historical `timestamp` | `time` |
| historical `tick_attribute_last`, `tick_attribute_bid_ask` | `attributes` |
| historical `TickLast::size`, `TickBidAsk::size_bid`, `size_ask` (`Option<f64>`) | `f64` |
| historical `TickMidpoint::size` | removed: TWS sends `0` on every historical midpoint tick and no size on a live one |
| `HistoricalTicksBuilder::trade()` | `last()` |

```rust,ignore
// 5.x
let mut volume = 0.0;
let trades = client.tick_by_tick(&contract, 0).last()?;
for trade in trades.iter_data() {
    let trade = trade?;
    volume += trade.size;
    let unreported = trade.trade_attribute.unreported;
}
let quotes = client.tick_by_tick(&contract, 0).bid_ask(IgnoreSize::No)?;
// quote.bid_price, quote.ask_size, quote.bid_ask_attribute
let history = client.historical_ticks(&contract, 100).trade()?;
// tick.timestamp, tick.tick_attribute_last, tick.size.unwrap_or(0.0)

// 6.0
let mut volume = 0.0;
let trades = client.tick_by_tick(&contract, 0).last()?;
for trade in trades.iter_data() {
    let trade = trade?;
    volume += trade.size;
    let unreported = trade.attributes.unreported;
}
let quotes = client.tick_by_tick(&contract, 0).bid_ask(IgnoreSize::No)?;
// quote.price_bid, quote.size_ask, quote.attributes
let history = client.historical_ticks(&contract, 100).last()?;
// tick.time, tick.attributes, tick.size
```

(`client` is a connected `Client`, `contract` a `Contract`.)

The compiler finds every use of a removed type, a renamed field or `trade()`, and every
historical size used as an `Option` or read from a midpoint. It does not catch three runtime changes:

- A historical trade or bid/ask size TWS did not send (or sent as the unset sentinel) reads as
  `0.0`, where 5.x gave `None`. TWS sent one on every tick probed, live and historical
  ([wire facts](wire-facts/market-data.md#tick-by-tick-sizes-and-ignoresize)).
- The types derive `Serialize` / `Deserialize`, and the keys change with the field names
  (`timestamp` -> `time`, `bid_price` -> `price_bid`, `trade_attribute` -> `attributes`,
  `mid_point` -> `price`), so JSON written by 5.x for them no longer deserializes. The `utoipa`
  schema names change to match.
- Code that only prints or serializes a historical size still compiles but changes output:
  `{:?}` gives `100.0` where it gave `Some(100.0)`, and a stored `null` size no longer
  deserializes.

### 40. `OptionChain::expirations` is `Vec<Expiry>`

In live checks of stock, index and futures option chains, TWS sent every expiration as a
`YYYYMMDD` date. The field was `Vec<String>`, so finding the nearest expiration meant parsing or
comparing text. It is now `Vec<Expiry>`: each date is an `Expiry::Date(ExpirationDate)`, which
goes straight into `Contract::option`. A `YYYYMM` entry is an `Expiry::Month` and an empty one
is dropped. Any other text is kept as `Expiry::Unknown`, so a value TWS adds later never ends the
stream. `Expiry` itself has no order, so `expirations.sort()` no longer compiles: take the dates
with `Expiry::date`, which order chronologically.

```rust,ignore
// 5.x
let mut expirations = chain.expirations.clone();
expirations.sort();
let nearest: ExpirationDate = expirations[0].parse()?;

// 6.0
let nearest = chain
    .expirations
    .iter()
    .filter_map(Expiry::date)
    .min(); // None if no dates
let text: Vec<String> = chain.expirations.iter().map(ToString::to_string).collect(); // the 5.x strings
```

The compiler finds every use as a `String`.

### 41. `ContractDetails::under_security_type` is `Option<SecurityType>`

TWS sends the underlying's security type as one of its codes, for example `STK` for a stock
option, `FUT` for a futures option and `IND` for a future on an index. It sends nothing when
the contract has no underlying (a stock or an index). The field was a `String`, empty when TWS
sent nothing. It is now `Option<SecurityType>`, `None` when TWS sends nothing; a code the crate
doesn't model is `SecurityType::Other`.

```rust,ignore
// 5.x
if details.under_security_type == "FUT" { /* futures option */ }

// 6.0
if details.under_security_type == Some(SecurityType::Future) { /* futures option */ }
let code = details.under_security_type.as_ref().map_or("", SecurityType::as_str); // the 5.x text
```

The compiler finds every use as a `String`. It doesn't catch the serialized form:
`ContractDetails` derives `Serialize` / `Deserialize`, and an unset field now serializes as
`null` where it was `""`. A set field is the wire code (`"STK"`, §47) as in 5.x, so 5.x JSON with
a code still deserializes; one with `""` doesn't.

### 42. `ContractDescription::derivative_security_types` is `Vec<SecurityType>`

`matching_symbols` lists the security types of the derivatives on each contract it finds. TWS
sends them as codes (`OPT`, `WAR`, `FUT`, `CFD`, ...), and the field was a `Vec<String>` of that
text. It is now `Vec<SecurityType>`. A code the crate doesn't model is `SecurityType::Other`, and
an empty entry is dropped.

```rust,ignore
// 5.x
if description.derivative_security_types.iter().any(|t| t == "OPT") { /* has options */ }
println!("{}", description.derivative_security_types.join(", "));

// 6.0
if description.derivative_security_types.contains(&SecurityType::Option) { /* has options */ }
let codes: Vec<&str> = description.derivative_security_types.iter().map(SecurityType::as_str).collect();
println!("{}", codes.join(", ")); // the 5.x text
```

The compiler finds every use as a `String`. `ContractDescription` doesn't derive `Serialize`,
so there is no serialized form to migrate.

### 43. `SecurityType` gains `StructuredProduct` and `EventContract`

TWS uses `IOPT` for Dutch warrants and structured products and `EC` for event contracts; both
appear among the derivative types `matching_symbols` reports (§42). 5.x decoded them as
`SecurityType::Other("IOPT")` and `Other("EC")`. They are now `SecurityType::StructuredProduct`
and `SecurityType::EventContract`, and they encode back to the same codes.

The compiler finds an exhaustive `match` on `SecurityType` that lacks the new arms. It doesn't
catch a comparison or match against `Other("IOPT")` or `Other("EC")`: that code still compiles
and no longer matches. Serialized, `SecurityType` is the wire code (§47), so a `Contract` with
one of these types writes `"IOPT"` or `"EC"` and reads back as the new variant.

### 44. `ExecutionFilter::time` is `Option<time::OffsetDateTime>`

`time` was a `String` passed to TWS as written. TWS rejected every zone suffix tried except
`UTC` and `Europe/London`, including the instrument's own zone (the form IB's error text gives
as its example), with error 10314 and no fills. The zone-less form drew a deprecation
warning (2174). The field now holds an instant, and the client sends it in UTC as
`yyyymmdd hh:mm:ss UTC`, dropping sub-seconds. `None` sends no time filter, as an empty
string did.

```rust,ignore
// 5.x
let filter = ExecutionFilter { time: "20261115-15:52:00".to_string(), ..Default::default() };

// 6.0
use time::macros::datetime;
let filter = ExecutionFilter { time: Some(datetime!(2026-11-15 15:52 UTC)), ..Default::default() };
```

The compiler finds every site that sets the field. A dashed `yyyymmdd-hh:mm:ss` string was
UTC, so it parses with
`time::PrimitiveDateTime::parse(text, time::macros::format_description!("[year][month][day]-[hour]:[minute]:[second]"))?.assume_utc()`.
As before, `time` filters within the days the filter covers and never extends them
([wire facts](wire-facts/orders.md#executiontime-rendering)).

### 45. `completed_orders`, `auto_open_orders` and `news_bulletins` take an enum, not a `bool`

The last three `bool` flags on `Client` methods are two-variant enums, as `LedgerOnly` (§16),
`SmartDepth` and `IgnoreSize` already are, so the call site says what the flag selects:

| Method (sync and async) | 5.x | 6.0 |
|---|---|---|
| `completed_orders` | `api_only: bool` | `api_only: orders::ApiOnly` |
| `auto_open_orders` | `auto_bind: bool` | `auto_bind: orders::AutoBind` |
| `news_bulletins` | `all_messages: bool` | `all_messages: news::AllMessages` |

`Yes` is the old `true` and `No` the old `false`. None of the three has a `Default`. `ApiOnly`
and `AutoBind` are in the prelude; `AllMessages` is imported from `ibapi::news`.

```rust,ignore
use ibapi::news::AllMessages;
use ibapi::orders::{ApiOnly, AutoBind};

// client: ibapi::client::blocking::Client (the async calls add `.await`)

// 5.x
let completed = client.completed_orders(true)?;
let orders = client.auto_open_orders(false)?;
let bulletins = client.news_bulletins(true)?;

// 6.0
let completed = client.completed_orders(ApiOnly::Yes)?;   // only orders placed through the API
let orders = client.auto_open_orders(AutoBind::No)?;      // new TWS orders are not bound to this client
let bulletins = client.news_bulletins(AllMessages::Yes)?; // all the existing bulletins for the current day
```

The compiler reports every call. Nothing changes on the wire or at runtime.

### 46. `ComboLeg::short_sale_slot` and `Order::delta_neutral_short_sale_slot` are `ShortSaleSlot`

Both were `i32` codes (`1` clearing broker, `2` third party) beside `Order::short_sale_slot`,
which is an `orders::ShortSaleSlot`. All three now hold the enum. A code the crate doesn't
model is `ShortSaleSlot::Unknown(code)` and is sent back unchanged. `ShortSaleSlot::None`
(code `0`) is still sent as no value; neither field is an `Option`.

```rust,ignore
use ibapi::orders::ShortSaleSlot;

// leg: ibapi::contracts::ComboLeg, order: ibapi::orders::Order

// 5.x
leg.short_sale_slot = 2;
order.delta_neutral_short_sale_slot = 1;
if leg.short_sale_slot == 0 { /* not a short sale */ }

// 6.0
leg.short_sale_slot = ShortSaleSlot::ThirdParty;
order.delta_neutral_short_sale_slot = ShortSaleSlot::Broker;
if leg.short_sale_slot == ShortSaleSlot::None { /* not a short sale */ }
let code = i32::from(leg.short_sale_slot); // where the number is wanted
```

The compiler finds every assignment and comparison; `ComboLeg { .., ..Default::default() }`
literals compile unchanged. Serialized, `ShortSaleSlot` is its wire code (§47), so the two fields keep
their 5.x form (`2`, or the bare number for an unmodeled code) and `ComboLeg` JSON written by 5.x
still deserializes. The compiler doesn't catch that `Order::short_sale_slot` changes the other
way: it serialized as the variant name (`"Broker"`, `{"Unknown":7}`) and is now the number. The
`utoipa` schema for all three fields is an integer.

### 47. Wire enums serialize in their wire form

5.x serialized most wire enums by their Rust variant name (`"Buy"`), with an unrecognized value in
the tagged form (`{"Unknown":3}`). Every one of them now serializes as the value TWS sends: a
string code as a string, an integer code as a bare number, and an unrecognized value as itself,
untagged. Renaming a variant no longer moves the JSON. The `utoipa` schemas, which listed the
variant names, change to match: a closed enum's `string` or `integer` lists its codes as `enum`
(`Action`: `["BUY", "SELL", "SSHORT", "SLONG"]`); an open enum's accepts any value, since it keeps
codes it doesn't list. The schemas describe the serialized form, so a service that validates
against them rejects the 5.x names `BarSize` still parses. The two `WhatToShow` schemas are the
components `HistoricalWhatToShow` and `RealtimeWhatToShow`.

| Enum | 5.x | 6.0 |
|---|---|---|
| `Action`, `LegAction`, `ExecutionFilterSide` | `"Buy"`, `"SellShort"` | `"BUY"`, `"SSHORT"` |
| `ExecutionSide` | `"Bought"` | `"BOT"` |
| `OptionRight` | `"Call"` | `"C"` |
| `SecurityType` | `"Stock"`, `{"Other":"X"}` | `"STK"`, `"X"` |
| `SecurityIdType` | `"Cusip"`, `{"Unknown":"X"}` | `"CUSIP"`, `"X"` |
| `Rule80A`, `OrderOpenClose` | `"Individual"`, `"Open"` | `"I"`, `"O"` |
| `FundAssetType`, `FundDistributionPolicyIndicator` | `"MoneyMarket"`, `"IncomeFund"` | `"001"`, `"Y"` |
| `market_data::historical::WhatToShow` | `"BidAsk"` | `"BID_ASK"` |
| `FaDataType`, `ServerLogLevel` | `"Groups"`, `"Info"` | `1`, `4` |
| `OcaType`, `OrderOrigin`, `VolatilityType`, `ReferencePriceType`, `WhatIfType` | `"CancelWithBlock"`, `{"Unknown":7}` | `1`, `7` |
| `ComboLegOpenClose`, `ArticleType` | `"Open"`, `"Unknown"`, `"Text"` | `1`, `3`, `0` |
| `TriggerMethod` on its own | `"Last"` | `2` |
| `DepthOperation`, `DepthSide`, `BulletinType`, `AdjustableTrailingUnit` | number (as an `i32` field) | number, unchanged from 5.x |
| `BarSize` | `"Min5"` | `"5 mins"`; `"Min5"` still loads |

`OrderType`, `TimeInForce`, `OrderStatusKind`, `ShortSaleSlot` and `Expiry` already serialized
this way, and `PriceCondition::trigger_method` was already a number. `TickType`,
`MarketDataType`, `Liquidity` and `OptionExerciseType` gain `Serialize` / `Deserialize`, as the
wire code, and `market_data::realtime::WhatToShow` as the wire string (`"MIDPOINT"`). `BarSize`
also parses its wire text now (`"5 mins".parse()`), and still parses the variant name in any
case (`"MIN5"`), so its 5.x JSON loads.

The compiler catches none of this. JSON written by 5.x for the enums in the first twelve rows does
not round-trip: a closed enum (`Action`, `OptionRight`, `LegAction`, the execution sides,
historical `WhatToShow`, `FaDataType`, `ServerLogLevel`) rejects a variant name, and every integer-coded
enum rejects a name or a tagged object. An open string enum (`SecurityType`, `SecurityIdType`,
`Rule80A`, `OrderOpenClose`, the fund enums) does *not* reject one: it keeps the name as an
unrecognized code. A `Contract` stored by 5.x with `"security_type":"Stock"` loads as
`SecurityType::Other("Stock")`, and a request built from it sends `Stock` to TWS. Convert stored JSON before loading it, by mapping each variant name to
its code (`as_str()` lists the codes for the string enums, `i32::from` for the integer ones).
