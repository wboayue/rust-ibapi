# Algo Order Builders - Remaining Types

Issue: https://github.com/wboayue/rust-ibapi/issues/345

## Completed (Core 4)

- [x] VWAP (`VwapBuilder`)
- [x] TWAP (`TwapBuilder`)
- [x] Percentage of Volume (`PctVolBuilder`)
- [x] Arrival Price (`ArrivalPriceBuilder`)

## Remaining Algo Types

### Priority 1 - Common Strategies

#### Adaptive Algo
Simple algo that combines smart routing with user-defined urgency.

```rust
pub struct AdaptiveBuilder {
    priority: Option<AdaptivePriority>,  // Urgent, Normal, Patient
}
```

Parameters:
- `adaptivePriority`: Urgent / Normal / Patient

#### Close Price (ClosePx)
Minimizes slippage relative to closing auction price.

```rust
pub struct ClosePriceBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    start_time: Option<String>,
    force_completion: Option<bool>,
}
```

Parameters:
- `maxPctVol` (0.1-0.5)
- `riskAversion`: Get Done / Aggressive / Neutral / Passive
- `startTime`
- `forceCompletion`

#### Dark Ice
Hidden order with randomized display sizes.

```rust
pub struct DarkIceBuilder {
    display_size: Option<i32>,
    start_time: Option<String>,
    end_time: Option<String>,
    allow_past_end_time: Option<bool>,
}
```

Parameters:
- `displaySize`
- `startTime`
- `endTime`
- `allowPastEndTime`

### Priority 2 - Advanced Strategies

#### Accumulate/Distribute (AD)
Slices orders into random increments at random intervals.

```rust
pub struct AccumulateDistributeBuilder {
    component_size: Option<i32>,
    time_between_orders: Option<i32>,  // seconds
    randomize_time_20: Option<bool>,
    randomize_size_55: Option<bool>,
    give_up: Option<i32>,
    catch_up: Option<bool>,
    wait_for_fill: Option<bool>,
    active_time_start: Option<String>,
    active_time_end: Option<String>,
}
```

Parameters:
- `componentSize`
- `timeBetweenOrders`
- `randomizeTime20`
- `randomizeSize55`
- `giveUp`
- `catchUp`
- `waitForFill`
- `activeTimeStart`
- `activeTimeEnd`

#### Balance Impact Risk
Balances market impact against adverse price movement risk.

```rust
pub struct BalanceImpactRiskBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    force_completion: Option<bool>,
}
```

Parameters:
- `maxPctVol` (0.1-0.5)
- `riskAversion`
- `forceCompletion`

#### Minimise Impact
Slices order to achieve market average with minimal impact.

```rust
pub struct MinimiseImpactBuilder {
    max_pct_vol: Option<f64>,
}
```

Parameters:
- `maxPctVol` (0.1-0.5)

### Priority 3 - Specialized PctVol Variants

#### Price Variant Percentage of Volume (PctVolPx)
Participation rate varies with market price.

```rust
pub struct PctVolPriceBuilder {
    pct_vol: Option<f64>,
    delta_pct_vol: Option<f64>,
    min_pct_vol_4_px: Option<f64>,
    max_pct_vol_4_px: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}
```

#### Size Variant Percentage of Volume (PctVolSz)
Participation rate varies based on remaining order size.

```rust
pub struct PctVolSizeBuilder {
    start_pct_vol: Option<f64>,
    end_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}
```

#### Time Variant Percentage of Volume (PctVolTm)
Participation rate varies over time.

```rust
pub struct PctVolTimeBuilder {
    start_pct_vol: Option<f64>,
    end_pct_vol: Option<f64>,
    start_time: Option<String>,
    end_time: Option<String>,
    no_take_liq: Option<bool>,
}
```

## Implementation Notes

1. All builders follow established pattern in `algo_builders.rs`
2. Add helper functions to `algo_helpers.rs`
3. Re-export new types from `builder/mod.rs`
4. Add docs to `docs/order-types.md`
5. Reuse existing enums (`RiskAversion`) where applicable

## References

- IB Algo docs: https://interactivebrokers.github.io/tws-api/ibalgos.html
- IB Algo parameters: https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm
