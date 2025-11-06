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
    .midprice(151.00)
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
// This will return an error - missing required limit price
let result = client.order(&contract)
    .buy(100)
    .limit(0.0)  // Invalid price
    .build();

match result {
    Err(ValidationError::InvalidPrice(price)) => {
        println!("Invalid price: {}", price);
    }
    _ => {}
}
```

## Best Practices

1. **Use specific order types:** Choose the most appropriate order type for your strategy
2. **Set time in force:** Always consider how long your order should remain active
3. **Handle errors:** Check for validation errors when building orders
4. **Test with paper trading:** Always test new order types in paper trading first
5. **Check product support:** Verify the order type is supported for your product type