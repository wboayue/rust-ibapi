---
id: macros-last-resort
title: Reach for macro_rules! only when ordinary Rust cannot express the pattern
cluster: style
status: active
triggers:
  - about to write a macro_rules!
  - N near-identical trait impls or test functions across types
  - reviewing a PR that adds a macro
symbols: [macro_rules, macro_use, macro_export, impl_str_partial_eq, impl_wire_enum]
related: [param-budget, exercise-production-code, fixture-builders]
precedents: ["#548", "#554"]
memory: [feedback_macro_repeated_trait_impls, feedback_no_speculative_test_infra, feedback_mirror_production_patterns]
---

Before writing a `macro_rules!`, ask whether a generic function with a trait bound, a
`for case in [(x, y), ...]` loop, or a plain helper would do. If yes, take that path.

A macro earns its cost in three cases:

1. **Shape-identical trait impls that a blanket impl cannot cover** — usually because the
   orphan rule blocks `impl<T: MyTrait> Display for T`.
2. **Bodies that depend on inherent items** — `<$t>::new`, `.as_str()`, struct-literal fields —
   where a test-only trait would cost more than the macro saves.
3. **Contexts where generics are not legal** — `const` initializers, generating a new nominal
   type per invocation, `prost` derive bodies.

Per-type `#[test]` granularity is **not** one of them. A generic `fn check_x<T: ...>(sample:
&str)` plus thin per-type `#[test] fn` wrappers gives the same independent pass/fail and the
same readable test names with no macro.

Crate-wide trait-impl macros live in `src/macros.rs`, reachable via `#[macro_use] mod macros;`
in `lib.rs`. Promote a module-local macro by copying it there. Never `#[macro_export]` — that
leaks it to the public API on docs.rs.

## Why

A macro is a second language in the file. Readers parse the invocation and then the expansion;
error spans point into generated code; goto-def and rust-analyzer degrade at the boundary. That
tax is worth paying to delete fifty lines of mechanical impls and not worth paying to save
five.

The live examples, and what each one buys:

| Macro | Invocations | Why not a function |
|---|---|---|
| `impl_str_partial_eq!` | 3 newtypes × 4 directions = 12 impls | Orphan rule: `impl PartialEq<Symbol> for str` cannot be blanket |
| `impl_wire_enum!` | 8 enums × 3 impls = 24 impls | Same — `Display` / `FromStr` / `ToField` on foreign traits. The `as_str` / `from_wire` data tables stay in normal Rust, visible to goto-def; only the plumbing expands |
| `string_newtype_surface!` | 5 test fns | Calls inherent `<$t>::new` and `.as_str()` |
| `single_req_id_request_builder!` | test fixtures | Generates a new named struct per proto type |

And the counter-examples, both from #554, which are the shape to imitate when a reviewer asks
"does this need to be a macro?":

- `check_serde_round_trip<T>` — a serde round-trip touches only trait methods
  (`Serialize` / `Deserialize` / `From<&str>`), so a generic function works.
- `check_str_partial_eq_round_trip<T>` — demoted from a macro even though the bound is heavy
  (`where T: PartialEq<str> + for<'a> PartialEq<&'a str> + ..., str: PartialEq<T>`). Ugly
  bounds are still cheaper than a macro.

## Reviewing a new macro

Challenge it twice. First, whether it can collapse into an existing one — #554 folded
`string_newtype_new_monomorphizations!` into `string_newtype_surface!`. Then, whether the
surviving macro's *body* actually needs a macro — the same pass initially missed
`serde_round_trip!` and demoted it only on a second look.

A test-side macro should also mirror its production counterpart's naming, and must have a
current consumer: matching an encoder file one-for-one is not a consumer. See
[fixture builders](../testing/fixture-builders.md).

## Precedents

- #548 — `impl_str_partial_eq!` collapsed 12 hand-written `PartialEq` impls into 3
  invocations, −55 lines. The canonical "macro earns it" case.
- #554 — the two demotions above, and the fold. The canonical "macro does not earn it" case.
