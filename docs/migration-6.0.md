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

### 10. `Missing` moves to `ibapi::builder`

`Missing`, the marker a typestate builder carries for a required field it has not been given, was `ibapi::contracts::Missing`. The order condition builders use it too now (§11), so it lives in a shared module, `ibapi::builder`. No other path is kept.

```rust,ignore
// 5.x
use ibapi::contracts::Missing;

// 6.0
use ibapi::builder::Missing;
```

Only code that names a builder's type needs the import, such as `OptionBuilder<Symbol, Missing, Missing>` in a signature.

### 11. Condition builders require a comparison before `build()`

`PriceConditionBuilder`, `TimeConditionBuilder`, `MarginConditionBuilder`, `VolumeConditionBuilder` and `PercentChangeConditionBuilder` panicked in `build()` when neither `greater_than()` nor `less_than()` had been called. They are now typestate builders: `build()`, and the `From<..> for OrderCondition` conversion that `OrderBuilder::condition`, `and_condition` and `or_condition` use, exist only after a comparison. A missing comparison is a compile error rather than a panic.

Code that sets a comparison before building compiles unchanged, through the builders and through the helpers in `orders::builder` (`price()`, `time()`, `margin()`, `volume()`, `percent_change()`):

```rust,ignore
let condition = price(265598, "SMART").greater_than(150.0).build(); // 5.x and 6.0
let condition = price(265598, "SMART").build();                     // 5.x: panicked; 6.0: does not compile
```

Each builder now takes a type parameter for its threshold, `Missing` until a comparison sets it (§10): `PriceConditionBuilder<f64>`, `TimeConditionBuilder<String>`, `MarginConditionBuilder<i32>`, `VolumeConditionBuilder<i32>`, `PercentChangeConditionBuilder<f64>`. Code that names a builder's type after the comparison writes it with the parameter; the bare name means the state before one.

`greater_than()` and `less_than()` can be called once per builder. A second call, which used to replace the first, no longer compiles.
