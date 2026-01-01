# Order Types Guide

This guide describes all order types supported by rust-ibapi and demonstrates how to create each type using the fluent API.

## Table of Contents

- [Basic Order Types](#basic-order-types)
  - [Market Order](#market-order)
  - [Limit Order](#limit-order)
  - [Stop Order](#stop-order)
  - [Stop-Limit Order](#stop-limit-order)
- [Trailing Orders](#trailing-orders)
  - [Trailing Stop](#trailing-stop)
  - [Trailing Stop Limit](#trailing-stop-limit)
- [Time-Based Orders](#time-based-orders)
  - [Market on Close (MOC)](#market-on-close-moc)
  - [Limit on Close (LOC)](#limit-on-close-loc)
  - [Market on Open](#market-on-open)
  - [Limit on Open](#limit-on-open)
  - [At Auction](#at-auction)
- [Conditional Orders](#conditional-orders)
  - [Market if Touched (MIT)](#market-if-touched-mit)
  - [Limit if Touched (LIT)](#limit-if-touched-lit)
- [Protected Orders](#protected-orders)
  - [Market with Protection](#market-with-protection)
  - [Stop with Protection](#stop-with-protection)
- [Advanced Market Orders](#advanced-market-orders)
  - [Market to Limit](#market-to-limit)
  - [Midprice Order](#midprice-order)
- [Pegged Orders](#pegged-orders)
  - [Relative/Pegged-to-Primary](#relativepegged-to-primary)
  - [Passive Relative](#passive-relative)
- [Special Order Types](#special-order-types)
  - [Discretionary Order](#discretionary-order)
  - [Sweep to Fill](#sweep-to-fill)
  - [Block Order](#block-order)
- [Complex Orders](#complex-orders)
  - [Bracket Orders](#bracket-orders)
  - [One-Cancels-All (OCA)](#one-cancels-all-oca)
- [Algorithmic Orders](#algorithmic-orders)
  - [VWAP](#vwap)
  - [TWAP](#twap)
  - [Percentage of Volume](#percentage-of-volume)
  - [Arrival Price](#arrival-price)

## Basic Order Types

### Market Order

A market order executes immediately at the best available price.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .submit()?;
```

**When to use:** When immediate execution is more important than price.

### Limit Order

A limit order executes at a specific price or better.

```rust
// Buy limit order - will only execute at $150.50 or lower
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.50)
    .submit()?;

// Sell limit order - will only execute at $155.00 or higher
let order_id = client.order(&contract)
    .sell(100)
    .limit(155.00)
    .submit()?;
```

**When to use:** When you want to control the execution price.

### Stop Order

A stop order becomes a market order when the stop price is reached.

```rust
// Buy stop - triggers when price rises to $152.00
let order_id = client.order(&contract)
    .buy(100)
    .stop(152.00)
    .submit()?;

// Sell stop (stop loss) - triggers when price falls to $148.00
let order_id = client.order(&contract)
    .sell(100)
    .stop(148.00)
    .submit()?;
```

**When to use:** For stop-loss orders or to enter positions on breakouts.

### Stop-Limit Order

A stop-limit order becomes a limit order when the stop price is reached.

```rust
// Stop at $148.00, then place limit order at $147.50
let order_id = client.order(&contract)
    .sell(100)
    .stop_limit(148.00, 147.50)
    .submit()?;
```

**When to use:** When you want to limit losses but also control the execution price.

## Trailing Orders

### Trailing Stop

A trailing stop adjusts the stop price as the market moves in your favor.

```rust
// Trailing stop with 5% trailing amount
let order_id = client.order(&contract)
    .sell(100)
    .trailing_stop(5.0, 95.00)  // 5% trailing, initial stop at $95
    .submit()?;
```

**When to use:** To protect profits while allowing for upside potential.

### Trailing Stop Limit

A trailing stop that becomes a limit order when triggered.

```rust
// Trailing stop limit with 5% trail and $0.50 limit offset
let order_id = client.order(&contract)
    .sell(100)
    .trailing_stop_limit(5.0, 95.00, 0.50)
    .submit()?;
```

**When to use:** When you want trailing protection with price control on execution.

## Time-Based Orders

### Market on Close (MOC)

Executes as a market order at or near the closing price.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market_on_close()
    .submit()?;
```

**When to use:** To execute at the day's closing price.
**Note:** Must be submitted before exchange cutoff time (typically 15-20 minutes before close).

### Limit on Close (LOC)

Executes as a limit order at the close, only if the closing price is at or better than the limit.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .limit_on_close(150.00)
    .submit()?;
```

**When to use:** To participate in closing auction with price protection.

### Market on Open

Executes as a market order at market open.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market_on_open()
    .submit()?;
```

**When to use:** To participate in opening auction.

### Limit on Open

Executes as a limit order at market open.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .limit_on_open(150.00)
    .submit()?;
```

**When to use:** To participate in opening auction with price protection.

### At Auction

Executes during pre-market auction period.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .at_auction(150.00)
    .submit()?;
```

**Products:** Futures (FUT), Stocks (STK)
**When to use:** For pre-market auction participation.

## Conditional Orders

### Market if Touched (MIT)

Becomes a market order when the trigger price is touched.

```rust
// Buy MIT - triggers when price falls to $145.00
let order_id = client.order(&contract)
    .buy(100)
    .market_if_touched(145.00)
    .submit()?;

// Sell MIT - triggers when price rises to $155.00
let order_id = client.order(&contract)
    .sell(100)
    .market_if_touched(155.00)
    .submit()?;
```

**When to use:** To enter positions when price reaches a specific level.
**Note:** Unlike stop orders, MIT buy orders trigger on price decline, sell orders on price rise.

### Limit if Touched (LIT)

Becomes a limit order when the trigger price is touched.

```rust
// Triggers at $145.00, then places limit order at $144.50
let order_id = client.order(&contract)
    .buy(100)
    .limit_if_touched(145.00, 144.50)
    .submit()?;
```

**When to use:** Similar to MIT but with price control on execution.

## Conditional Orders with Conditions

Conditional orders allow you to specify market conditions that must be met before an order is activated. You can combine multiple conditions using AND/OR logic to create sophisticated trading strategies.

### Available Condition Types

1. **Price Condition** - Trigger when a contract reaches a specific price
2. **Time Condition** - Trigger at or after a specific time
3. **Margin Condition** - Trigger based on account margin cushion percentage
4. **Execution Condition** - Trigger when a specific contract executes
5. **Volume Condition** - Trigger when trading volume reaches a threshold
6. **Percent Change Condition** - Trigger when price changes by a percentage

### Single Condition Example

```rust
use ibapi::orders::builder::price;

// Buy when AAPL price exceeds $150
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .condition(price(265598, "SMART").greater_than(150.0))
    .submit()?;
```

### Multiple AND Conditions

Use `.and_condition()` to require all conditions to be met:

```rust
use ibapi::orders::builder::{price, margin, time};

// Buy only when ALL conditions are true:
// - Price > $150 AND
// - Margin cushion > 30% AND
// - Time after 2:30 PM ET
let order_id = client.order(&contract)
    .sell(50)
    .limit(155.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(margin().greater_than(30))
    .and_condition(time().greater_than("20251230 14:30:00 US/Eastern"))
    .submit()?;
```

### Multiple OR Conditions

Use `.or_condition()` to trigger when any condition is met:

```rust
use ibapi::orders::builder::{price, volume};

// Buy when EITHER condition is true:
// - Price < $100 OR
// - Volume > 50 million
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .condition(price(265598, "SMART").less_than(100.0))
    .or_condition(volume(265598, "SMART").greater_than(50_000_000))
    .submit()?;
```

### Mixed AND/OR Logic

Combine AND and OR logic for complex strategies:

```rust
use ibapi::orders::builder::{price, margin, time, volume};

// Logic: (price > 10 AND margin < 20) OR time > X OR volume > Y
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .condition(price(123445, "SMART").greater_than(10.0))
    .and_condition(margin().less_than(20))
    .or_condition(time().greater_than("20251010 09:30:00 US/Eastern"))
    .or_condition(volume(123445, "SMART").greater_than(10_000_000))
    .submit()?;
```

### Condition Types Reference

#### Price Condition

Monitor price movements and trigger when threshold is crossed:

```rust
use ibapi::orders::builder::price;
use ibapi::orders::conditions::TriggerMethod;

// Trigger when price goes above $150
let condition = price(265598, "SMART")
    .greater_than(150.0)
    .trigger_method(TriggerMethod::Last);
```

**Parameters:**
- `contract_id` - Contract ID (get via `contract_details()`)
- `exchange` - Exchange to monitor (e.g., "SMART", "NASDAQ")
- `.greater_than(price)` - Trigger when price rises above threshold
- `.less_than(price)` - Trigger when price falls below threshold
- `.trigger_method(method)` - Which price to use (see [`TriggerMethod`])

**Available Trigger Methods:**
- `TriggerMethod::Default` - Last for most securities, double bid/ask for OTC and options
- `TriggerMethod::DoubleBidAsk` - Two consecutive bid or ask prices
- `TriggerMethod::Last` - Last traded price
- `TriggerMethod::DoubleLast` - Two consecutive last prices
- `TriggerMethod::BidAsk` - Current bid or ask price
- `TriggerMethod::LastOrBidAsk` - Last price or bid/ask if no last price
- `TriggerMethod::Midpoint` - Mid-point between bid and ask

#### Time Condition

Trigger at or after a specific date/time:

```rust
use ibapi::orders::builder::time;

// Trigger after 2:30 PM Eastern on Dec 30, 2025
let condition = time()
    .greater_than("20251230 14:30:00 US/Eastern");

// Trigger before market close
let condition = time()
    .less_than("20251230 16:00:00 US/Eastern");
```

**Time Format:** `"YYYYMMDD HH:MM:SS TZ"`
- Example: `"20251230 14:30:00 US/Eastern"`
- Common timezones: `US/Eastern`, `US/Central`, `UTC`

#### Margin Condition

Trigger based on account margin cushion percentage:

```rust
use ibapi::orders::builder::margin;

// Trigger when margin cushion falls below 30%
let condition = margin().less_than(30);

// Trigger when margin cushion exceeds 50%
let condition = margin().greater_than(50);
```

**Use case:** Risk management - close positions when margin is low, or enter new positions when margin is high.

**Calculation:** Margin cushion = (Equity with Loan Value - Maintenance Margin) / Net Liquidation Value

#### Execution Condition

Trigger when a specific contract trades in your account:

```rust
use ibapi::orders::builder::execution;

// Trigger when MSFT executes (any fill, any side)
let condition = execution("MSFT", "STK", "SMART");
```

**Parameters:**
- `symbol` - Contract symbol
- `security_type` - "STK", "OPT", "FUT", etc.
- `exchange` - Exchange to monitor

**Use case:** Pairs trading - automatically hedge after initial position fills.

#### Volume Condition

Trigger when cumulative daily volume reaches threshold:

```rust
use ibapi::orders::builder::volume;

// Trigger when TSLA volume exceeds 50 million shares
let condition = volume(76792991, "SMART")
    .greater_than(50_000_000);

// Trigger when volume is below average
let condition = volume(76792991, "SMART")
    .less_than(10_000_000);
```

**Note:** Volume resets daily at market open.

**Use case:** Enter positions only after sufficient liquidity is established.

#### Percent Change Condition

Trigger based on percentage price change from session open:

```rust
use ibapi::orders::builder::percent_change;

// Trigger when SPY moves up more than 2% from open
let condition = percent_change(756733, "SMART")
    .greater_than(2.0);

// Trigger on 3% decline
let condition = percent_change(756733, "SMART")
    .less_than(-3.0);
```

**Note:** Percentage is a decimal (2.0 = 2%, not 0.02). Resets at session open.

**Use case:** Momentum trading - enter on significant moves.

### Advanced Features

#### Ignoring Regular Trading Hours

By default, conditions are only evaluated during regular trading hours. To monitor conditions outside RTH:

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market()
    .condition(price(265598, "SMART").greater_than(150.0))
    .conditions_ignore_rth()  // Monitor during extended hours
    .submit()?;
```

#### Cancel vs Activate on Condition

By default, orders activate when conditions are met. You can reverse this to cancel orders:

```rust
// Cancel order if not filled by 3:30 PM
let order_id = client.order(&contract)
    .buy(100)
    .limit(149.0)
    .condition(time().greater_than("20251230 15:30:00 US/Eastern"))
    .conditions_cancel_order()  // Cancel instead of activate
    .submit()?;
```

### Getting Contract IDs

Most conditions require contract IDs. Get them via `contract_details()`:

```rust
// Get contract details to find contract ID
let details = client.contract_details(&Contract::stock("AAPL")).next()?;
let contract_id = details.contract.contract_id;

// Now use in conditions
let condition = price(contract_id, "SMART").greater_than(150.0);
```

### How Conditions Work

**Conjunction Logic:**
- Each condition has an `is_conjunction` flag that determines how it combines with the NEXT condition
- `.condition()` - First condition, always uses AND logic
- `.and_condition()` - Sets previous condition to AND with this one
- `.or_condition()` - Sets previous condition to OR with this one

**Evaluation:**
- Conditions are evaluated continuously during market hours (or extended hours if configured)
- Once conditions are met, the order activates immediately
- After activation, the original conditional order is replaced with a regular order

### Best Practices

1. **Test with paper trading first** - Conditional orders can be complex
2. **Get contract IDs before trading** - Cache contract IDs to avoid repeated lookups
3. **Use time conditions for time-limited opportunities** - Automatically cancel stale orders
4. **Combine price and margin conditions for risk management** - Don't trade when margin is low
5. **Monitor extended hours carefully** - Use `conditions_ignore_rth()` only when needed
6. **Keep conditions simple** - Complex logic can be hard to debug
7. **Use execution conditions for hedging** - Automatically place offsetting trades

### Complete Example: Risk-Managed Entry

```rust
use ibapi::orders::builder::{price, margin, time, volume};

// Enter position only when:
// 1. Price is favorable (> $150)
// 2. Sufficient liquidity (volume > 80M)
// 3. After market stabilizes (> 10:00 AM)
// 4. Account has adequate margin (> 40%)
let order_id = client.order(&contract)
    .buy(100)
    .limit(151.0)
    .condition(price(265598, "SMART").greater_than(150.0))
    .and_condition(volume(265598, "SMART").greater_than(80_000_000))
    .and_condition(time().greater_than("20251230 10:00:00 US/Eastern"))
    .and_condition(margin().greater_than(40))
    .good_till_cancel()
    .submit()?;
```

## Protected Orders

### Market with Protection

Market order with protection against extreme price movements.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market_with_protection()
    .submit()?;
```

**Products:** Futures
**When to use:** For futures trading when you want market execution with some price protection.

### Stop with Protection

Stop order with protection against extreme price movements.

```rust
let order_id = client.order(&contract)
    .sell(100)
    .stop_with_protection(148.00)
    .submit()?;
```

**Products:** Futures
**When to use:** For stop-loss orders in futures with price protection.

## Advanced Market Orders

### Market to Limit

Starts as a market order, unfilled portion becomes a limit order.

```rust
let order_id = client.order(&contract)
    .buy(100)
    .market_to_limit()
    .submit()?;
```

**When to use:** To get immediate partial fills while limiting price on remainder.

### Midprice Order

Seeks to fill at the midpoint between bid and ask.

```rust
// Midprice order with price cap at $151.00
let order_id = client.order(&contract)
    .buy(100)
    .midprice(Some(151.00))
    .submit()?;

// Midprice order without price cap
let order_id = client.order(&contract)
    .buy(100)
    .midprice(None)
    .submit()?;
```

**Requirements:** TWS 975+, US stocks with Smart routing only
**When to use:** To potentially get better prices than the spread.

## Pegged Orders

### Relative/Pegged-to-Primary

Seeks more aggressive price than the National Best Bid/Offer (NBBO).

```rust
// Buy with $0.05 more aggressive than best bid, cap at $150.50
let order_id = client.order(&contract)
    .buy(100)
    .relative(0.05, Some(150.50))
    .submit()?;

// Without price cap
let order_id = client.order(&contract)
    .buy(100)
    .relative(0.05, None)
    .submit()?;
```

**Products:** CFD, STK, OPT, FUT
**When to use:** For aggressive order placement relative to NBBO.

### Passive Relative

Seeks less aggressive price than NBBO for better queue position.

```rust
// Sell with $0.05 less aggressive offset
let order_id = client.order(&contract)
    .sell(100)
    .passive_relative(0.05)
    .submit()?;
```

**Products:** STK, WAR
**When to use:** To join the queue with passive pricing.

## Special Order Types

### Discretionary Order

Limit order with hidden discretionary amount for price improvement.

```rust
// Visible limit at $50.00, willing to pay up to $50.10
let order_id = client.order(&contract)
    .buy(100)
    .discretionary(50.00, 0.10)
    .submit()?;
```

**Products:** STK only
**When to use:** To hide your true price willingness from the market.

### Sweep to Fill

Prioritizes speed of execution over price, sweeping through multiple price levels.

```rust
let order_id = client.order(&contract)
    .sell(500)
    .sweep_to_fill(49.95)
    .submit()?;
```

**Products:** CFD, STK, WAR
**When to use:** When immediate execution of large orders is critical.

### Block Order

For large option orders with minimum 50 contracts.

```rust
let order_id = client.order(&option_contract)
    .buy(100)
    .block(2.50)
    .submit()?;
```

**Products:** OPT (ISE exchange only)
**Requirements:** Minimum 50 contracts
**When to use:** For large option trades seeking better execution.

## Complex Orders

### Bracket Orders

Creates a parent order with attached take-profit and stop-loss orders.

```rust
let bracket_ids = client.order(&contract)
    .buy(100)
    .bracket()
    .entry_limit(50.00)    // Entry price
    .take_profit(55.00)    // Take profit at $55
    .stop_loss(45.00)      // Stop loss at $45
    .submit_all()?;

// Access individual order IDs
println!("Parent order: {}", bracket_ids.parent);
println!("Take profit: {}", bracket_ids.take_profit);
println!("Stop loss: {}", bracket_ids.stop_loss);
```

**When to use:** To automatically manage risk and profit targets.

### One-Cancels-All (OCA)

Groups orders where execution of one cancels all others.

```rust
// Build individual orders with OCA group
let order1 = client.order(&contract1)
    .buy(100)
    .limit(50.00)
    .oca_group("MyOCA", 1)
    .build()?;

let order2 = client.order(&contract2)
    .buy(100)
    .limit(45.00)
    .oca_group("MyOCA", 1)
    .build()?;

// Submit as OCA group (requires contract-order pairs)
let order_ids = client.submit_oca_orders(vec![
    (contract1.clone(), order1),
    (contract2.clone(), order2)
])?;
```

**When to use:** When you want multiple entry strategies but only one execution.

## Algorithmic Orders

IB provides several algorithmic order strategies that automatically slice orders over time to minimize market impact. These algos are available for most US stocks and can significantly improve execution quality for large orders.

### VWAP

Volume Weighted Average Price seeks to achieve the VWAP from order submission to market close.

```rust
use ibapi::orders::builder::vwap;

let order_id = client.order(&contract)
    .buy(1000)
    .limit(150.0)
    .algo(vwap()
        .max_pct_vol(0.2)
        .start_time("09:00:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .allow_past_end_time(true)
        .no_take_liq(true)
        .build()?)
    .submit()?;
```

**Parameters:**
- `max_pct_vol(0.1-0.5)` - Maximum participation rate as % of volume
- `start_time` - Start time (format: "HH:MM:SS TZ")
- `end_time` - End time (format: "HH:MM:SS TZ")
- `allow_past_end_time` - Continue trading after end time
- `no_take_liq` - Passive only, don't take liquidity
- `speed_up` - Speed up execution in momentum

**When to use:** For large orders where you want to match the market's volume-weighted average price.

### TWAP

Time Weighted Average Price slices orders evenly over time.

```rust
use ibapi::orders::builder::{twap, TwapStrategyType};

let order_id = client.order(&contract)
    .buy(1000)
    .limit(150.0)
    .algo(twap()
        .strategy_type(TwapStrategyType::Marketable)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()?)
    .submit()?;
```

**Parameters:**
- `strategy_type` - Execution style:
  - `Marketable` - Cross the spread when needed
  - `MatchingMidpoint` - Execute at midpoint
  - `MatchingSameSide` - Stay on one side of spread
  - `MatchingLast` - Match last traded price
- `start_time` - Start time
- `end_time` - End time
- `allow_past_end_time` - Continue after end time

**When to use:** For even distribution of execution across time.

### Percentage of Volume

Controls participation rate relative to market volume.

```rust
use ibapi::orders::builder::pct_vol;

let order_id = client.order(&contract)
    .buy(1000)
    .limit(150.0)
    .algo(pct_vol()
        .pct_vol(0.1)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .no_take_liq(true)
        .build()?)
    .submit()?;
```

**Parameters:**
- `pct_vol(0.1-0.5)` - Target participation rate
- `start_time` - Start time
- `end_time` - End time
- `no_take_liq` - Passive only

**When to use:** To limit market impact while participating at a consistent rate.

### Arrival Price

Targets the bid/ask midpoint at the time of order arrival.

```rust
use ibapi::orders::builder::{arrival_price, RiskAversion};

let order_id = client.order(&contract)
    .buy(1000)
    .limit(150.0)
    .algo(arrival_price()
        .max_pct_vol(0.1)
        .risk_aversion(RiskAversion::Neutral)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .force_completion(true)
        .build()?)
    .submit()?;
```

**Parameters:**
- `max_pct_vol(0.1-0.5)` - Maximum participation rate
- `risk_aversion` - Urgency level:
  - `GetDone` - Complete quickly
  - `Aggressive` - Favor speed over price
  - `Neutral` - Balance speed and price
  - `Passive` - Favor price over speed
- `start_time` - Start time
- `end_time` - End time
- `force_completion` - Complete by end time
- `allow_past_end_time` - Continue after end time

**When to use:** When you want to benchmark against arrival price.

### Manual Algo Order Construction

For custom algo strategies or parameters not exposed by the builders, you can construct orders manually:

```rust
use ibapi::orders::{Order, Action, TagValue};

let order = Order {
    order_type: "LMT".to_string(),
    action: Action::Buy,
    total_quantity: 1000.0,
    lmt_price: Some(150.0),
    algo_strategy: "Vwap".to_string(),
    algo_params: vec![
        TagValue { tag: "maxPctVol".to_string(), value: "0.2".to_string() },
        TagValue { tag: "startTime".to_string(), value: "09:00:00 US/Eastern".to_string() },
        TagValue { tag: "endTime".to_string(), value: "16:00:00 US/Eastern".to_string() },
    ],
    ..Default::default()
};

let order_id = client.next_order_id();
client.place_order(order_id, &contract, &order)?;
```

### Algo Order Best Practices

1. **Use limit prices** - Always set a limit price as a safety cap
2. **Consider market hours** - Most algos work best during regular trading hours
3. **Start conservative** - Use lower participation rates initially (0.1-0.2)
4. **Monitor execution** - Review fills to calibrate future algo parameters
5. **Test with small orders** - Validate algo behavior before large trades

## Order Modifiers

### Time in Force

Control how long an order remains active:

```rust
// Good till canceled
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .good_till_cancel()
    .submit()?;

// Good till specific date
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .good_till_date("20241231 23:59:59")
    .submit()?;

// Fill or kill - must fill entirely immediately or cancel
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .fill_or_kill()
    .submit()?;

// Immediate or cancel - fill what's possible, cancel remainder
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .immediate_or_cancel()
    .submit()?;
```

### Trading Hours

```rust
// Allow execution outside regular trading hours
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .outside_rth()
    .submit()?;

// Regular hours only (default)
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .regular_hours_only()
    .submit()?;
```

### Hidden Orders

```rust
// Hide order from market depth (NASDAQ only)
let order_id = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .hidden()
    .submit()?;
```

**Note:** Hidden orders only work for NASDAQ-routed orders.

## What-If Orders

Analyze margin and commission impact without placing the order:

```rust
let analysis = client.order(&contract)
    .buy(100)
    .limit(150.00)
    .what_if()
    .analyze()?;

println!("Initial margin: {}", analysis.init_margin);
println!("Maintenance margin: {}", analysis.maint_margin);
println!("Commission: {}", analysis.commission);
```

## Important Notes

1. **Exchange Support:** Not all order types are supported on all exchanges
2. **Product Restrictions:** Some order types are limited to specific products (stocks, options, futures, etc.)
3. **Time Restrictions:** Orders like MOC/LOC have submission cutoff times
4. **Minimum Requirements:** Block orders require minimum quantities
5. **TWS Version:** Some features like Midprice orders require specific TWS versions

## Error Handling

The fluent API validates orders at build time:

```rust
// This will return an error - invalid price (NaN)
let result = client.order(&contract)
    .buy(100)
    .limit(f64::NAN)  // Invalid price
    .build();

match result {
    Err(ValidationError::InvalidPrice(price)) => {
        println!("Invalid price: {}", price);
    }
    _ => {}
}
```

## Custom Order Construction

While the fluent API provides a convenient interface for creating common order types, you can also manually construct `Order` objects for more advanced or specialized scenarios. Orders can be submitted using either `place_order` or `submit_order` methods.

```rust
use rust_ibapi::orders::Order;

// Manually construct an order
let order = Order {
    action: Action::Buy,
    order_type: "LMT".to_string(),
    total_quantity: 100.0,
    lmt_price: Some(150.50),
    tif: "GTC".to_string(),
    outside_rth: true,
    ..Default::default()
};

// Get the next valid order ID
let order_id = client.next_order_id();

// Submit using place_order (returns subscription for updates)
let subscription = client.place_order(order_id, &contract, &order)?;

// Or using submit_order (fire-and-forget, no subscription)
client.submit_order(order_id, &contract, &order)?;
```

This approach is useful when:
- You need to set order fields not exposed by the fluent API
- You're migrating code from other TWS API implementations
- You need maximum control over order construction
- You're implementing custom order strategies

**Note:** When manually constructing orders, you're responsible for ensuring all required fields are set correctly and the combination is valid for the target exchange and product type.

## Best Practices

1. **Use specific order types:** Choose the most appropriate order type for your strategy
2. **Set time in force:** Always consider how long your order should remain active
3. **Handle errors:** Check for validation errors when building orders
4. **Test with paper trading:** Always test new order types in paper trading first
5. **Check product support:** Verify the order type is supported for your product type