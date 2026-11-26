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
precedents: ["#1169", "#1249"]
---

Ask who fixes it. A runtime value a caller may correct (a quantity, price or strike, a date from
user or file input, a field on a caller-built `Order`) fails with a typed
`ibapi::builder::ValidationError` variant: returned bare from a constructor or `FromStr`
(`Result<Self, ValidationError>`), wrapped as `Error::Validation` by anything returning
`ibapi::Error`. A caller bug no one recovers from at runtime (a builder missing a required field,
a nonsense duration, bad client config, `buffer_limit(0)`) is `Error::InvalidArgument(String)`.
Add a `ValidationError` variant rather than formatting a value into a string. If the type
already carries type parameters, prefer typestate to either error.

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
comparison (`docs/migration-6.0.md` §12). It's the better fix where a builder already has
type parameters.

## Precedents

- #1169 — order builders' `submit` / `analyze` / `build_order` moved from `InvalidArgument`
  text to `Error::Validation(ValidationError)`; the split was still per module.
- #1249 — the rule stated: `ValidationError` moved to `ibapi::builder`, gained `InvalidField` and
  `InvalidDate`. `ext_operator`, `ExpirationDate` / `ContractMonth` / `Expiry` construction and
  parsing, `Strike::new`, and option / `ContractBuilder` strike and expiry checks moved to it.
  Required-field checks, `Contract::spread`, `historical_data` ranges and `ClientBuilder`
  stayed `InvalidArgument`. `ContractBuilder` now runs `Strike::new` too, so it rejects the
  zero / NaN strikes it used to pass.
