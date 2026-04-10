# Split Client Methods into Domain Modules

## Problem

`client/sync.rs` (4833 lines) and `client/async.rs` (4726 lines) are monolithic. Every new API method modifies these files. Each Client method is a thin one-line delegation to a standalone function in the domain module — two functions for every operation.

## Approach

Move Client methods into domain modules as `impl Client` blocks, **eliminating the standalone functions**. The implementation logic moves directly into the `impl Client` method body. Remove the now-unnecessary `pub mod blocking` re-exports and module-level function re-exports from domain `mod.rs` files.

## Per-Domain Changes

### Pattern (using accounts as example)

**Before** — two functions per operation:
```rust
// client/sync.rs
impl Client {
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        accounts::blocking::positions(self)  // thin delegation
    }
}

// accounts/sync.rs
pub fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    request_helpers::blocking::shared_subscription(client, ...)  // real logic
}

// accounts/mod.rs
pub mod blocking {
    pub use super::sync::{positions, ...};
}
```

**After** — one function:
```rust
// accounts/sync.rs
impl Client {
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        request_helpers::blocking::shared_subscription(self, ...)  // real logic, no indirection
    }
}

// accounts/mod.rs — blocking re-exports removed
```

### Each Domain Module (`sync.rs` and `async.rs`)

1. **Convert standalone functions to `impl Client` methods** — change `fn foo(client: &Client, ...)` to `pub fn foo(&self, ...)`, replace `client.` with `self.`
2. **Move doc comments and examples** from `client/sync.rs` methods onto the new `impl Client` methods
3. **Keep non-function items** — `StreamDecoder` impls, `SharesChannel` impls stay as-is

### Domain `mod.rs` Cleanup

Remove from each domain `mod.rs`:
- `pub mod blocking { pub use super::sync::{ ... }; }` block
- `pub use sync::{ ... };` re-exports of standalone functions
- `pub use r#async::{ ... };` re-exports of standalone functions

**Files affected:**
| Module | `mod.rs` has blocking/re-exports |
|---|---|
| `accounts/mod.rs` | lines 340-357 |
| `orders/mod.rs` | line 1589+ |
| `contracts/mod.rs` | line 932+ |
| `market_data/historical/mod.rs` | lines 559-610 |
| `market_data/realtime/mod.rs` | lines 507-515 |
| `news/mod.rs` | lines 101-110 |
| `scanner/mod.rs` | line 112+ |
| `display_groups/mod.rs` | lines 24-32 |
| `wsh/mod.rs` | line 67+ |

### `client/mod.rs` Cleanup

Remove from `client/mod.rs`:
- `pub mod blocking` — no longer needed (domain modules don't export standalone functions)

Wait — `client/mod.rs` `blocking` re-exports `Client` itself and subscription types, not domain functions. **Keep `client/mod.rs` as-is.** Users still need `use ibapi::client::blocking::Client`.

### `client/sync.rs` — What Gets Removed

All domain method `impl Client` blocks (with doc comments) — roughly lines 287-2083. What remains (~800 lines):
- `struct Client`
- Core `impl Client`: connect, accessors, builder entry points (`order()`, `market_data()`), `pub(crate)` internals
- `impl Drop`, `impl Debug`
- `SharesChannel` re-export + Subscription doc block
- Core tests (connect, server_time, client_id)

### `client/async.rs` — Same treatment

## What Moves Where

| Domain Module | Methods (sync) | Count |
|---|---|---|
| `accounts/sync.rs` | positions, positions_multi, pnl, pnl_single, account_summary, account_updates, account_updates_multi, managed_accounts, family_codes | 9 |
| `contracts/sync.rs` | contract_details, cancel_contract_details, market_rule, matching_symbols, calculate_option_price, calculate_implied_volatility, option_chain | 7 |
| `orders/sync.rs` | all_open_orders, auto_open_orders, cancel_order, completed_orders, executions, global_cancel, open_orders, place_order, submit_order, order_update_stream, exercise_options | 11 |
| `market_data/historical/sync.rs` | head_timestamp, historical_data, historical_data_streaming, historical_schedules, historical_schedules_ending_now, historical_ticks_bid_ask, historical_ticks_mid_point, historical_ticks_trade, cancel_historical_ticks, histogram_data | 10 |
| `market_data/realtime/sync.rs` | realtime_bars, tick_by_tick_all_last, tick_by_tick_bid_ask, tick_by_tick_last, tick_by_tick_midpoint, switch_market_data_type, market_depth, market_depth_exchanges | 8 |
| `news/sync.rs` | news_providers, news_bulletins, historical_news, news_article, contract_news, broad_tape_news | 6 |
| `scanner/sync.rs` | scanner_parameters, scanner_subscription | 2 |
| `display_groups/sync.rs` | subscribe_to_group_events | 1 |
| `wsh/sync.rs` | wsh_metadata, wsh_event_data_by_contract, wsh_event_data_by_filter | 3 |

Same for async modules.

## Tests

Tests from `client/sync.rs` move to domain modules alongside their methods:
- Account tests -> `accounts/sync.rs`
- Contract tests -> `contracts/sync.rs`
- Order tests -> `orders/sync.rs`
- Market data tests -> split between `historical/sync.rs` and `realtime/sync.rs`
- News tests -> `news/sync.rs`
- Scanner tests -> `scanner/sync.rs`
- WSH tests -> `wsh/sync.rs`
- Core tests stay in `client/sync.rs`

Same for async tests.

## Breaking Changes

Removing standalone function re-exports is a **breaking change** for users who call domain functions directly (e.g., `ibapi::accounts::blocking::positions(&client)`). These are eliminated in favor of `client.positions()`.

## Execution Order

1. Sync — one domain at a time, verify compilation after each
2. Async — same approach
3. Final: `cargo clippy --all-features && just test`

## Verification

```bash
cargo fmt
cargo clippy
cargo clippy --no-default-features --features sync
cargo clippy --all-features
just test
```
