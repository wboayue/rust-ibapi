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
related: [param-budget, fixture-builders]
precedents: ["#548", "#554"]
memory: [feedback_macro_repeated_trait_impls, feedback_no_speculative_test_infra, feedback_mirror_production_patterns]
---

Before writing a `macro_rules!`, ask whether a generic function with a trait bound, a
`for case in [(x, y), ...]` loop, or a plain helper would do. If yes, use it.

A macro earns its cost in three cases:

1. **Shape-identical trait impls a blanket impl cannot cover**, usually because the orphan rule
   blocks `impl<T: MyTrait> Display for T`.
2. **Bodies that depend on inherent items** (`<$t>::new`, `.as_str()`, struct-literal fields)
   where a test-only trait would cost more than the macro saves.
3. **Contexts where generics are not legal**: `const` initializers, generating a new nominal
   type per invocation, naming a distinct `prost` struct per call.

Per-type `#[test]` granularity is **not** one of them. A generic `fn check_x<T: ...>(sample: &str)`
plus thin per-type `#[test] fn` wrappers gives the same independent pass/fail and test names.

Crate-wide macros live in `src/macros.rs`, reachable via `#[macro_use] mod macros;` in `lib.rs`;
promote a module-local macro by moving it there. Never `#[macro_export]`: it leaks to the public
API on docs.rs.

## Why

A macro is a second language in the file: readers parse invocation and expansion, error spans
point into generated code, goto-def degrades at the boundary. Worth it to delete fifty lines of
mechanical impls, not to save five.

The macros in `src/` today (`grep -rn 'macro_rules!' src --include=*.rs`):

| Macro | Home | Why not a function |
|---|---|---|
| `impl_str_partial_eq!` | `macros.rs` | Case 1: `impl PartialEq<Symbol> for str` cannot be blanket. 3 newtypes × 4 directions |
| `impl_wire_enum!` | `macros.rs` | Case 1: `Display` / `FromStr` / `ToField` are foreign traits. The `as_str` / `from_wire` tables stay in plain Rust; only the plumbing expands |
| `string_newtype_surface!` | `contracts/types_tests.rs` | Case 2: calls inherent `<$t>::new` and `.as_str()` |
| `single_req_id_request_builder!`, `request_id_response_builder!`, `empty_request_builder!` | `testdata/builders/mod.rs` | Case 3: a new named struct per fixture |
| `encode_cancel_by_id!`, `encode_empty_proto!` | `proto/encoders.rs` | Case 3: a distinct `proto::` struct literal per call; no trait covers "prost message with a `req_id` field" |
| `impl_proto_payload!` | `proto/payload.rs` | Case 3: a `const MESSAGE_ID` per `prost` type. Load-bearing: `expect_proto` will not compile for a payload without a line |

The counter-examples to imitate, both from #554:

- `check_serde_round_trip<T>` — touches only trait methods, so a generic function works.
- `check_str_partial_eq_round_trip<T>` — demoted from a macro despite heavy bounds
  (`T: PartialEq<str> + for<'a> PartialEq<&'a str> + ..., str: PartialEq<T>`). Ugly bounds are
  still cheaper than a macro.

## Reviewing a new macro

Challenge it twice: can it fold into an existing macro, and does the surviving macro's body
actually need to be one? A test-side macro should mirror its production counterpart's naming and
have a current consumer; see [fixture builders](../testing/fixture-builders.md).

## Precedents

- #548 — `impl_str_partial_eq!` collapsed 12 hand-written `PartialEq` impls into 3 invocations,
  −55 lines. The canonical "macro earns it" case.
- #554 — folded `string_newtype_new_monomorphizations!` into `string_newtype_surface!`, demoted
  `check_str_partial_eq_round_trip`, and on a second look `serde_round_trip!`. The canonical
  "macro does not earn it" case.
