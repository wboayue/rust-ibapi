---
id: clock-seams
title: Extract pure *_from(now) helpers from time-dependent code
cluster: testing
status: active
triggers:
  - a function reads OffsetDateTime::now_utc() and then branches
  - coverage on a module stalls around 60-70%
  - a test would only pass on certain calendar days
  - production reads an env var or random value and branches on it
symbols: [next_friday_from, third_friday_from, front_from, next_quarter_from, OffsetDateTime, date-macro]
related: [coverage-floor]
precedents: ["#554"]
memory: [feedback_coverage_mop_up_tactics]
---

A function that reads the clock and then branches on the result is structurally untestable —
date-driven arms fire only on specific calendar days, so coverage caps around 60-70%.

Keep the public method as a one-line wrapper that fetches the clock and delegates to a private
`*_from(today: Date)` helper holding the branching logic. Tests call the helper with literal
dates:

```rust
// production: ExpirationDate::third_friday_of_month() reads the clock, delegates
let tf = ExpirationDate::third_friday_from(date!(2025 - 03 - 22));
assert_eq!(format!("{}", tf), "20250418"); // past the third Friday, rolls a month
```

Use `time::macros::date!` for literal dates, not `Date::from_calendar_date(..).unwrap()` — it
is the project standard (`src/lib_tests.rs`, `examples/async/wsh_event_data_by_contract.rs`,
and many `datetime!` callsites in `src/messages/tests.rs`).

## Why

The seam is the function that takes the value as a parameter. Everything non-deterministic
belongs on one side of it and all the logic on the other, so tests can drive every arm without
waiting for a Friday.

`src/contracts/types.rs` is the worked example, across two types. `ExpirationDate` carries
`next_friday_from` / `third_friday_from`, which take a `Date`; `ContractMonth` carries
`front_from` / `next_quarter_from`, which take `(year, month, day)`. Either shape works — the
point is that the clock read happens in the caller. The helpers stay private; the sibling
`types_tests.rs` reaches them through `use super::*;`.

The same structure applies to any non-deterministic read that production then branches on —
env vars, random generators — not just time.

## Precedents

- #554 — lifted `contracts/types.rs` from 61.4% to 99.6% line coverage. The clock seams were
  the structural half; see [coverage-floor](coverage-floor.md) for the serde and
  monomorphization mop-up that finished it.
