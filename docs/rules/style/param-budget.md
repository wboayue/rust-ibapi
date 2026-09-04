---
id: param-budget
title: Three parameters is the budget; a fourth needs a builder or a grouping struct
cluster: style
status: active
triggers:
  - writing a function that takes four or more parameters
  - deciding whether a signature should become a builder
  - adding #[allow(clippy::too_many_arguments)]
  - a public method takes several Option arguments
symbols: [too_many_arguments]
related: [domain-module-layout, macros-last-resort, doc-parity-audit]
precedents: ["#549", "#573", "#660", "#752", "#792"]
memory: [feedback_rule19_builder_fed_helpers_exception, feedback_magic_none_split_to_builder]
---

Three parameters is the budget. The receiver does not count — `fn foo(&self, a, b, c)` is at
the limit, `fn foo(&self, a, b, c, d)` is over it.

Over the budget, pick by what the arguments *are*:

- **Any argument optional or defaultable** → fluent builder. This is the case the rule exists
  for: it spares callers from `foo(id, None, None, None, None)`.
- **All arguments required, no reasonable default** → group the related ones into a struct —
  a `DateRange { start, end }` for the `start_time` / `end_time` pairs would be the obvious
  first one, though no such type exists yet — or keep the flat signature with a comment
  saying why.
  `client.foo(a, b, c, d, e)` is no worse than `client.foo(a).b(b).c(c).d(d).e(e).run()`; a
  mechanical builder conversion buys nothing.

`pub(crate)` helpers called from a builder's finalisers are the documented exception — the
public surface is already a builder, and the flat-arg helper is the deliberate seam to the wire
encoder. Annotate with `#[allow(clippy::too_many_arguments)]` and say so in a comment; the four
such sites in `market_data/historical/` are the pattern to copy.

## Why

Positional arguments of the same type are the failure mode: `(id, start, end, limit)` swaps
silently, and `(id, None, None, None, None)` tells a reader nothing about what was defaulted.
Named setters make both cases loud.

**Clippy does not enforce this.** `clippy::too_many_arguments` has a default threshold of 7, so
it only fires at eight or more — more than twice the budget. Every `#[allow]` in the tree is
therefore at 8+ args, and a 4- to 7-arg signature passes CI silently. This is a review-time
rule with no gate behind it.

An `Option`-heavy public signature is usually two problems at once. Before building a setter
for an `Option<T>` argument, check whether the `Option` should exist at all — if the wire
allows `None` but every real caller passes `Some(x)`, drop it. When a magic-`None` API does get
split, `foo(Option<T>)` → `foo(T)` + `foo_default()` → builder is the planned three-step path;
the split is a waypoint, not the destination. Both sub-rules live in
[doc parity audit](../docs/doc-parity-audit.md).

The open violation inventory — four internal helpers and one client method, with a
per-signature verdict on each — lives in
[plans/code-consistency-followups.md](../../../plans/code-consistency-followups.md). Take one
when you are already in the file.

## Precedents

- #660 — the `pegged_to_benchmark` free function became `PeggedToBenchmark::new(action,
  quantity, starting_price)` plus six setters for the optional fields. The clear-win shape:
  three required args at the entry point, everything optional named.
- #549 — the order-construction sweep, which is what a fluent builder buys at the call site
  (`.buy(100).limit(150.0).submit()`), and where builder enum coverage has to be audited
  against the underlying enum.
- #573 — a signature reshape that dropped `Option<WhatToShow>` to `WhatToShow` after checking
  the wire, the C# client, and every caller. Removing an argument beats wrapping it.
- #752 — `wsh_event_data_by_contract` / `_by_filter`: one required argument plus two to four
  `Option`s that every caller passed as `None`, converted to builders with one setter each.
- #792 — `option_chain(symbol, exchange, security_type, contract_id)`: four required-looking
  arguments where `exchange` documented `""` as a default. The audit had filed it as
  `Option<Exchange>`, not a builder; a live probe before the change showed the field is
  TWS's `futFopExchange` (any named exchange returns an empty chain for a stock) and that
  `contract_id = 0`, which two examples passed, is a hard rejection. Probe the "meaningful
  default" before deciding its shape — the probe, not the signature, was the interesting part.
