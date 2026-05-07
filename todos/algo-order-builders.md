# Algo Order Builders - Remaining Types

Issue: https://github.com/wboayue/rust-ibapi/issues/345 (closed — core 4 satisfied the original ask; remaining algos are follow-on enhancement)

## Completed (Core 4)

- [x] VWAP (`VwapBuilder`, strategy `"Vwap"`)
- [x] TWAP (`TwapBuilder`, strategy `"Twap"`)
- [x] Percentage of Volume (`PctVolBuilder`, strategy `"PctVol"`)
- [x] Arrival Price (`ArrivalPriceBuilder`, strategy `"ArrivalPx"`)

## Remaining Algo Types

### Priority 1 - Common Strategies

#### Adaptive Algo (strategy `"Adaptive"`)
Simple algo that combines smart routing with user-defined urgency.

```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum AdaptivePriority {
    Urgent,
    #[default]
    Normal,
    Patient,
}

impl AdaptivePriority {
    fn as_str(&self) -> &'static str {
        match self {
            AdaptivePriority::Urgent => "Urgent",
            AdaptivePriority::Normal => "Normal",
            AdaptivePriority::Patient => "Patient",
        }
    }
}

pub struct AdaptiveBuilder {
    priority: Option<AdaptivePriority>,
}
```

Parameters:
- `adaptivePriority`: Urgent / Normal / Patient

#### Close Price (strategy `"ClosePx"`)
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
- `maxPctVol` (0.1-0.5) — reuse `validate_pct_vol`
- `riskAversion`: reuse existing `RiskAversion` enum
- `startTime`
- `forceCompletion`

#### Dark Ice (strategy `"DarkIce"`)
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

#### Accumulate/Distribute (strategy `"AD"`)
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

#### Balance Impact Risk (strategy `"BalanceImpactRisk"`)
Balances market impact against adverse price movement risk.

```rust
pub struct BalanceImpactRiskBuilder {
    max_pct_vol: Option<f64>,
    risk_aversion: Option<RiskAversion>,
    force_completion: Option<bool>,
}
```

Parameters:
- `maxPctVol` (0.1-0.5) — reuse `validate_pct_vol`
- `riskAversion` — reuse existing enum
- `forceCompletion`

#### Minimise Impact (strategy `"MinImpact"`)
Slices order to achieve market average with minimal impact.

```rust
pub struct MinimiseImpactBuilder {
    max_pct_vol: Option<f64>,
}
```

Parameters:
- `maxPctVol` (0.1-0.5) — reuse `validate_pct_vol`

### Priority 3 - Specialized PctVol Variants

#### Price Variant Percentage of Volume (strategy `"PctVolPx"`)
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

#### Size Variant Percentage of Volume (strategy `"PctVolSz"`)
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

#### Time Variant Percentage of Volume (strategy `"PctVolTm"`)
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
2. Add helper functions to `algo_helpers.rs` (one per builder, e.g. `adaptive()`, `close_price()`, …)
3. Re-export new types and helpers from `builder/mod.rs`
4. Add docs to `docs/order-types.md`, and update the TOC at lines 40-42 (currently lists VWAP/TWAP only — also missing PctVol and ArrivalPrice; fix in same PR)
5. Reuse existing enums (`RiskAversion`) and helpers (`validate_pct_vol`, `MIN_PCT_VOL`, `MAX_PCT_VOL`, `bool_param`) where applicable
6. **Doc-examples (rule 23)**: every public builder struct, `new()`, and `algo_helpers` entry point gets a runnable `# Examples` block. Mirror the existing `VwapBuilder` / `vwap()` shape
7. **Modernize touched module (rules 13/14)**: `algo_builders.rs` still has an inline `#[cfg(test)] mod tests` block at lines 572-724. The PR that adds new algos should also extract those tests to a sibling `algo_builders_tests.rs` and wire it in via `#[cfg(test)] #[path = "algo_builders_tests.rs"] mod tests;`. New builders' tests go in the same sibling file
8. **Tests for each new builder**: per CLAUDE.md rule 11, every new `pub fn`/builder needs a unit test — round-trip through `build()` and assert `params` tag/value pairs, plus boundary tests for percentage validators

## References

- IB Algo docs: https://interactivebrokers.github.io/tws-api/ibalgos.html
- IB Algo parameters: https://www.interactivebrokers.com/en/software/api/apiguide/tables/ibalgo_parameters.htm
- C# reference: `/Users/wboayue/projects/tws-api/source/csharpclient/client/Order.cs` (search `AlgoStrategy`)
