# Migration Guide: 5.x to 6.0

This guide covers the breaking changes in `ibapi` 6.0, which is in development on `main`. Each
breaking change adds its section here in the PR that makes it. For 4.x → 5.0, see
[`migration-5.0.md`](migration-5.0.md).

## Breaking changes

Planned changes are tracked on the
[6.0 milestone](https://github.com/wboayue/rust-ibapi/milestone/11).

### 1. `ComboLegOpenClose` and `MarketDataType` keep unrecognized codes

Both mapped any code they did not model to a unit `Unknown`, losing the value. `ComboLegOpenClose::Unknown` was itself wire code `3`, so a leg TWS reported with code `4` or higher was sent back as `3` when the contract was reused in an order. Both now take the shape the order enums have had since 4.0 (`Liquidity`, `OcaType`, ...):

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

### 2. `From<i32> for ArticleType` is removed

It mapped any unknown code to `ArticleType::Text`, so a binary article type TWS added later would have read as text. `news_article` has not used it since 5.4.0: its decoder rejects an unknown code with `Error::Parse`. Nothing else in the crate called it. Match on the two variants directly; there is no replacement conversion.
