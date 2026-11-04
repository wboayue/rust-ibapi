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
