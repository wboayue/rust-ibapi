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

Keep the public method a one-line wrapper that reads the clock and delegates to a private
`*_from(today)` helper holding the branching logic. Tests call the helper with literal dates:

```rust
// production: ExpirationDate::third_friday_of_month() reads the clock, delegates
let tf = ExpirationDate::third_friday_from(date!(2025 - 03 - 22));
assert_eq!(format!("{}", tf), "20250418"); // past the third Friday, rolls a month
```

Write literal dates with `time::macros::date!`, not `Date::from_calendar_date(..).unwrap()`
(`grep -rlE '\bdate!|datetime!' src examples` for usages).

## Why

A function that reads the clock and then branches is untestable: date-driven arms fire only
on specific calendar days, so coverage caps around 60-70%. The seam is the function that takes
the value as a parameter — nondeterminism on one side, all logic on the other. The same applies
to env vars and random values that production branches on.

`src/contracts/types.rs` is the worked example. `ExpirationDate::next_friday_from` /
`third_friday_from` take a `Date`; `ContractMonth::front_from` / `next_quarter_from` take
`(year, month, day)`. Either shape works. The helpers stay private; `types_tests.rs` reaches
them through `use super::*;`.

## Precedents

- #554 — `contracts/types.rs` 61.4% → 99.6% line coverage; clock seams were the structural
  half, [coverage-floor](coverage-floor.md) mop-up the rest.
