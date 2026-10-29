---
id: param-budget
title: Three parameters is the budget; a fourth needs a builder or a grouping struct
cluster: style
status: active
triggers:
  - writing a public function that takes four or more parameters
  - deciding whether a signature should become a builder
  - adding #[allow(clippy::too_many_arguments)]
  - a public method takes several Option arguments
symbols: [too_many_arguments]
related: [domain-module-layout, macros-last-resort, doc-parity-audit]
precedents: ["#549", "#573", "#660", "#752", "#792", "#901", "#1079", "#1123"]
memory: [feedback_rule19_builder_fed_helpers_exception, feedback_magic_none_split_to_builder]
---

Three parameters is the budget for the **public API** (anything rustdoc shows). The receiver
does not count: `fn foo(&self, a, b, c)` is at the limit. `pub(crate)`, `pub(in …)` and private
functions are out of scope and need no annotation at four to seven arguments.

Over budget, pick by what the arguments *are*:

- **Any argument optional or defaultable** → fluent builder. This is what the rule exists for:
  sparing callers `foo(id, None, None, None, None)`.
- **All required, no reasonable default** → group related ones into a struct (no `DateRange`
  for `start_time` / `end_time` pairs exists yet; it would be the obvious first), or keep the
  flat signature with a comment saying why. `client.foo(a, b, c, d, e)` is no worse than
  `client.foo(a).b(b).c(c).d(d).e(e).run()`.

Internal functions are exempt because every caller is in the tree, so reshaping one never breaks
a user. Most are seams between a public builder's finalizer and the wire encoder. At eight or
more arguments clippy forces `#[allow(clippy::too_many_arguments)]`; add a one-line reason, as the
sites in `market_data/historical/` do.

## Why

Same-typed positional arguments swap silently (`(id, start, end, limit)`), and
`(id, None, None, None, None)` tells a reader nothing. Named setters make both loud.

**Clippy does not enforce this.** `too_many_arguments` defaults to 7, so it fires only at eight;
a 4- to 7-arg public signature passes CI. This is review-time only.

An `Option`-heavy public signature is often two problems. Before adding a setter for an
`Option<T>`, check whether the `Option` should exist: if every real caller passes `Some(x)`, drop
it. A magic-`None` split, `foo(Option<T>)` → `foo(T)` + `foo_default()` → builder, is a waypoint,
not the destination. Both live in [doc parity audit](../docs/doc-parity-audit.md).

Public signatures over budget are all-required and kept flat with a reason: `historical_news`
(five arguments), `Contract::option` (superseded by `Contract::call()` / `put()`), and the
client-less `orders::order_builder` free functions (module doc explains).

## Precedents

- #1079 — scoped the budget to the public API; annotating the internal 4–7-arg functions would
  have been churn with no caller to protect.
- #901 — removed free functions whose same-typed prices were ordered differently from the fluent
  method of the same name (`stop_limit(.., limit, stop)` vs `.stop_limit(stop, limit)`) in favor
  of `Order::builder()`. Fix an API split by finishing one builder, not adding one per function.
  Where named fields are the point and no builder fits, a plain struct argument (`BracketPrices`,
  `Adjustment`) does it.
- #1123 — `SpreadBuilder::butterfly(ButterflyLegs)` / `iron_condor(IronCondorLegs)`: the
  struct form brought back the 4-id `iron_condor` that #901 removed for silent swaps. Same-typed
  ids whose order changes the strategy get named fields even at three (butterfly is in budget);
  a symmetric pair (`straddle(call, put)`, both bought) stays positional.
- #660 — `pegged_to_benchmark` free function → `PeggedToBenchmark` builder, later folded into
  `Order::builder()` by #901.
- #549 — order construction converged on the fluent builder (`.buy(100).limit(150.0).submit()`);
  see [builder enum coverage](builder-enum-coverage.md).
- #573 — dropped `Option<WhatToShow>` to `WhatToShow` after checking the wire, the C# client,
  and every caller. Removing an argument beats wrapping it.
- #752 — `wsh_event_data_by_contract` / `_by_filter`: one required argument plus `Option`s every
  caller passed as `None`, converted to builders.
- #792 — `option_chain(symbol, exchange, security_type, contract_id)` → builder. A live probe
  showed `exchange` is TWS's `futFopExchange` (a named exchange returns an empty chain for a
  stock) and `contract_id = 0` is rejected. Probe the "meaningful default" before choosing a shape.
