---
id: enum-typing
title: Verify the wire before typing a String field as an enum
cluster: wire
status: active
triggers:
  - typing a String field as an enum
  - a decoder falls back to T::default() on a missing field
  - adding a FromStr impl for a wire value
symbols: [parse_required, parse_optional, FromStr, impl_wire_enum, Error::Parse]
related: [proto-only-decoding, fixture-builders]
precedents: ["#518", "#556", "#558", "#559", "#647"]
memory: [feedback_verify_wire_before_typing, feedback_helper_signature_precursor_pr, feedback_test_fixture_display_cruft, feedback_live_diagnostic_tests]
---

Before typing a `String` field as an enum, **verify the wire actually carries enumerated
values** — grep captured-wire fixtures and check the C# reference at
`/Users/wboayue/projects/tws-api/source/csharpclient/client/`. Field-name resemblance to a
known vocabulary is misleading.

Once verified: strict enum, `Display` round-trips back to the IB wire string, `FromStr`
returns `Result<_, Error>`. Decode with the generic helpers in `src/proto/decoders.rs`:

```rust
parse_required::<OrderStatusKind>(proto.status.as_deref(), "status")?   // -> Result<T, Error>
parse_optional::<OptionRight>(proto.right.as_deref())?                  // -> Result<Option<T>, Error>
```

`parse_required` takes a label for the error message; `parse_optional` does not. Each new
enum only needs `impl FromStr<Err = Error>` — no per-field wrapper.

**The decoder must reject empty or missing input as `Error::Parse`, never fall back to
`T::default()`.**

## Why

A silent `T::default()` masks an incomplete TWS response. The field reads as a plausible
value, the monitoring loop never sees an error, and the subscription hangs waiting for state
that already arrived malformed.

The verification step matters because the wire is not what field names suggest.
`OrderState.completed_status` sounds enumerated; the wire carries free-form text like
`"Cancelled by Trader"`. `FundamentalReportType` is documented with six values; TWS accepts
four. Typing either strictly from the name alone produces a decoder that rejects valid data.

When a new strict decoder rejects an existing test fixture, **do not broaden `FromStr` to
accept the literal.** Check `/Users/wboayue/projects/tws-api/samples/` first. PR #559 found
that `right: "?"` was a VB sample app's display fallback for empty string, never real TWS
wire — broadening would have baked a display artifact into the parser.

For shape-identical enums, `impl_wire_enum!` in `src/macros.rs` generates `Display`,
`FromStr<Err = Error>`, and `ToField` from an `as_str` + `from_wire` data table.

## Required tests

- Both `None` and `Some("")` produce `Err(Error::Parse(..))` for required fields, or
  `Ok(None)` for optional ones.
- A `Display` / `FromStr` round-trip over the full variant table.

## Precedents

`docs/migration-3.0.md` §5, §9–§12, §23, §29, §30 is the shipped-outcome ledger for these
migrations — consult it rather than re-deriving which fields were converted and why.

- #518 — established the pattern (`OrderStatus.status` → `OrderStatusKind`).
- #556, #558 — established the generic `parse_required` / `parse_optional` shape.
- #559 — the `right: "?"` fixture-vs-wire lesson.
- #647 — `FundamentalReportType`: docs listed six values, TWS accepts four.

