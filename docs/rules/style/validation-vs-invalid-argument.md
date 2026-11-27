---
id: validation-vs-invalid-argument
title: Bad values are Validation, caller bugs are InvalidArgument
cluster: style
status: active
triggers:
  - returning an error for a bad input from a builder, constructor, FromStr or client method
  - adding a check on a field of a caller-built Order, Contract or request
  - about to write Error::InvalidArgument(format!(..))
symbols: [Error::Validation, Error::InvalidArgument, ValidationError, ibapi::builder]
related: [builder-enum-coverage, param-budget]
precedents: ["#1169", "#1249", "#1331", "#PRB"]
---

Ask who fixes it. A runtime value a caller may correct (a quantity, price or strike, a date from
user or file input, a field on a caller-built `Order`) fails with a typed
`ibapi::builder::ValidationError` variant: returned bare from a constructor or `FromStr`
(`Result<Self, ValidationError>`), wrapped as `Error::Validation` by anything returning
`ibapi::Error`. A caller bug no one recovers from at runtime (a builder missing a required field,
conflicting `historical_data` settings, bad client config, `buffer_limit(0)`) is
`Error::InvalidArgument(String)`.
Add a `ValidationError` variant rather than formatting a value into a string. For a required
builder field, prefer typestate (`ibapi::builder::Missing`) to either error; `InvalidArgument`
is for what the type can't track, such as a price that depends on the `OrderType` value passed
to `OrderBuilder::order_type(..)`. `ValidationError` has no missing-field variant.

`Error::Parse` is for text that came from TWS: decoders, and the `impl_wire_enum!` and
`BarTimestamp` parsers they call. A `FromStr` whose text comes from the caller returns
`ValidationError`: `Duration`, `BarSize`, historical `WhatToShow`, and realtime `WhatToShow`
through the `impl_wire_enum!` `caller_input` form. Use that form for a new wire enum no decoder
reads. The expiry types return `ValidationError` too, though `Expiry::from_wire` also parses TWS
text with them: it turns a failure into `Expiry::Unknown`, so no decode error surfaces.
`ValidationError` is `#[non_exhaustive]`, so a new variant is not a breaking change.

## Why

Before #1249 the split was by module, not by meaning: order builders returned `Validation`,
everything else `InvalidArgument`. So one call could fail both ways. `place_order` returned
`Validation` for a bad quantity built through `OrderBuilder` but `InvalidArgument` for a bad
`ext_operator`. A caller re-prompting for a mistyped expiry had to match on message text. A
value that came from a user or a file needs a variant to match on, while a caller bug only needs
a message that says what to change, because no one catches it.

`ValidationError` lives in `ibapi::builder`, not `orders::builder`, because it is the payload of
the crate-wide `Error::Validation` and contracts use it too. A constructor that returns
`ValidationError` instead of `ibapi::Error` gives the narrowest type. `?` still converts through
`#[from]`, so callers that return `ibapi::Error` see no difference.

Typestate removes the error path entirely, as the condition builders did for a missing
comparison (`docs/migration-6.0.md` §12) and the order builders for a side, an order type or a
bracket leg (§51). `OrderBuilder` gained two parameters for it: a builder with dozens of optional
fields still pays for typestate only in the setters that change the state.

## Precedents

- #1169 — order builders' `submit` / `analyze` / `build_order` moved from `InvalidArgument`
  text to `Error::Validation(ValidationError)`; the split was still per module.
- #1249 — the rule stated: `ValidationError` moved to `ibapi::builder`, gained `InvalidField` and
  `InvalidDate`. `ext_operator`, `ExpirationDate` / `ContractMonth` / `Expiry` construction and
  parsing, `Strike::new`, and option / `ContractBuilder` strike and expiry checks moved to it.
  Required-field checks, `Contract::spread`, `historical_data` ranges and `ClientBuilder`
  stayed `InvalidArgument`. `ContractBuilder` now runs `Strike::new` too, so it rejects the
  zero / NaN strikes it used to pass. Review caught that zero was over-reach: TWS reads
  strike 0 as unset and resolves the contract from its id (`docs/wire-facts/contracts.md`),
  so zero still passes with a `contract_id` or `local_symbol`.
- #1331 — `Duration`, `BarSize` and historical and realtime `WhatToShow` `FromStr` moved from
  `Error::Parse` to `ValidationError::InvalidField`: no decoder calls them, so the text is always
  caller input. Review caught realtime `WhatToShow`, which shares the wire-enum macro but is never
  decoded; the macro gained a `caller_input` form for it. The decoded wire enums stayed on
  `Error::Parse`. `ValidationError` became `#[non_exhaustive]`.
- #PRB — `OrderBuilder` (side, order type), `BracketOrderBuilder` (entry,
  take-profit, stop-loss) and `ExerciseOptionsBuilder` (exercise / lapse) became typestate and
  `ValidationError::MissingRequiredField` was removed. What the type can't see stayed a runtime
  check, now `InvalidArgument`: `order_type(..)` without the price its value needs,
  `pegged_to_benchmark` without `reference_contract`, `GoodTillDate` without a date.
  `OrderBuilder::build()` moved from `ValidationError` to `ibapi::Error` to carry both.
