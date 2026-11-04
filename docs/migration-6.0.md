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
