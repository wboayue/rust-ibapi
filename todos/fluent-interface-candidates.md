# Client Methods - Fluent Interface Candidates

## Methods that Could Benefit from Fluent Interface

### 1. **Market Data Methods**
These methods have multiple optional parameters that could be better expressed with a builder pattern:

#### Current API:
```rust
// market_data - has 4 parameters, 2 booleans
client.market_data(&contract, &["233"], false, false)?

// realtime_bars - has 4 parameters
client.realtime_bars(&contract, BarSize::Sec5, WhatToShow::Trades, TradingHours::Regular)?

// tick_by_tick methods - have 3 parameters
client.tick_by_tick_all_last(&contract, 100, false)?
```

#### Proposed Fluent Interface:
```rust
// Market data with builder
let subscription = client.market_data(&contract)
    .generic_ticks(&["233", "236"])
    .snapshot()
    .subscribe()?;

// Realtime bars
let bars = client.realtime_bars(&contract)
    .bar_size(BarSize::Sec5)
    .what_to_show(WhatToShow::Trades)
    .extended_hours()  // or .regular_hours()
    .subscribe()?;

// Tick by tick
let ticks = client.tick_by_tick(&contract)
    .all_last()  // or .bid_ask() or .midpoint() or .last()
    .max_ticks(100)
    .include_size()
    .subscribe()?;
```

### 2. **Historical Data Methods**
These have many parameters with different combinations:

#### Current API:
```rust
client.historical_data(
    &contract, 
    Some(end_time), 
    Duration::Days(30), 
    BarSize::Day1, 
    WhatToShow::Trades,
    TradingHours::Regular
)?

client.historical_ticks_bid_ask(&contract, start_time, end_time, 1000, true, false)?
```

#### Proposed Fluent Interface:
```rust
// Historical data
let data = client.historical_data(&contract)
    .end_time(end_time)  // optional, defaults to now
    .duration(Duration::Days(30))
    .bar_size(BarSize::Day1)
    .what_to_show(WhatToShow::Trades)
    .regular_hours()  // or .extended_hours()
    .fetch()?;

// Historical ticks
let ticks = client.historical_ticks(&contract)
    .bid_ask()  // or .trades() or .midpoint()
    .from(start_time)
    .to(end_time)
    .max_ticks(1000)
    .use_rth()
    .ignore_size()
    .fetch()?;
```

### 3. **Order Placement Methods**
Order placement could benefit from validation and better ergonomics:

#### Current API:
```rust
let mut order = Order::default();
order.action = OrderAction::Buy;
order.order_type = OrderType::Limit;
order.total_quantity = 100.0;
order.lmt_price = Some(150.0);

client.submit_order(order_id, &contract, &order)?
```

#### Proposed Fluent Interface:
```rust
client.order(&contract)
    .buy(100)  // or .sell(100)
    .limit(150.0)  // or .market() or .stop(145.0)
    .time_in_force(TimeInForce::Day)
    .outside_rth()
    .submit()?;  // or .place() for subscription
```

### 4. **Account/Position Queries**
These methods with optional parameters:

#### Current API:
```rust
client.positions_multi(Some(&account), Some(&model))?
client.pnl_single(&account, contract_id, Some(&model))?
```

#### Proposed Fluent Interface:
```rust
// Positions with optional filters
let positions = client.positions()
    .account(&account)
    .model(&model)
    .subscribe()?;

// PnL with options
let pnl = client.pnl(&account)
    .single(contract_id)  // or omit for account-level
    .model(&model)
    .subscribe()?;
```

### 5. **WSH Event Data**
Complex queries with many optional parameters:

#### Current API:
```rust
client.wsh_event_data_by_contract(
    contract_id, 
    Some(start_date), 
    Some(end_date), 
    Some(100), 
    Some(AutoFill::Future)
)?
```

#### Proposed Fluent Interface:
```rust
let events = client.wsh_events()
    .by_contract(contract_id)  // or .by_filter("...")
    .from(start_date)
    .to(end_date)
    .limit(100)
    .auto_fill(AutoFill::Future)
    .fetch()?;
```

### 6. **Scanner Subscription**
Scanner has complex filter setup:

#### Current API:
```rust
let mut subscription = ScannerSubscription::default();
// ... many field assignments
let filters = vec![TagValue::new("tag1", "value1")];
client.scanner_subscription(&subscription, &filters)?
```

#### Proposed Fluent Interface:
```rust
let scanner = client.scanner()
    .instrument("STK")
    .location("STK.US.MAJOR")
    .scan_code("TOP_PERC_GAIN")
    .above_price(5.0)
    .below_price(100.0)
    .above_volume(1000000)
    .filter("tag1", "value1")
    .filter("tag2", "value2")
    .limit(50)
    .subscribe()?;
```

## Summary of Benefits

1. **Better Discoverability**: IDE autocomplete shows available options at each step
2. **Type Safety**: Invalid combinations can be prevented at compile time
3. **Cleaner Code**: No need for mutable variables or default structs
4. **Optional Parameters**: Much clearer which parameters are optional
5. **Validation**: Builders can validate state before executing
6. **Backwards Compatibility**: Can coexist with existing API

## Priority Ranking

Methods that would benefit most are those with:
- Multiple optional parameters (historical data, WSH events)
- Boolean flags that could be methods (market data snapshot, trading hours)
- Complex configuration objects (orders, scanner subscriptions)
- Multiple variants of similar functionality (tick types, PnL single vs account)

### High Priority
1. **Order Placement** - Most complex, error-prone API
2. **Market Data** - Very commonly used with confusing boolean parameters
3. **Historical Data** - Many optional parameters and variants

### Medium Priority
4. **Scanner Subscription** - Complex but less frequently used
5. **Account/Position Queries** - Optional parameters but simpler API

### Low Priority
6. **WSH Event Data** - Specialized use case
7. **Other methods** - Simple enough or rarely used