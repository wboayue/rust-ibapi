---
id: enum-typing
title: Verify the wire before typing a String field as an enum
cluster: wire
status: active
triggers:
  - typing a String field as an enum
  - a decoder falls back to T::default() on a missing field
  - adding a FromStr impl for a wire value
symbols: [parse_required, parse_optional, FromStr, impl_wire_enum, Error::Parse, Unknown]
related: [proto-only-decoding, fixture-builders]
precedents: ["#518", "#556", "#558", "#559", "#647", "#774", "#822", "#825"]
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

An unrecognized **value** is a separate question from a missing one. On a streaming
decode path — where a parse failure terminates the subscription — a value IBKR adds
later must not become a client-side outage, so such enums are *open*: a non-empty
unrecognized string parses as a value-preserving `Unknown(String)` variant via
`impl_wire_enum!(Name, fallback Unknown)`, while empty stays `Error::Parse`. Keep
matching exact and case-sensitive — a case-variant lands in `Unknown` where it is
observable, never coerced to the nearest known variant. The variant costs `Copy` and
makes `as_str` return `&str`; the enum stays deliberately exhaustive so the compiler
points callers at the new arm. Two consequences the derive path hides when the enum's
serde form is the wire string, as `OrderStatusKind`'s is: serde must be hand-written (the
derive would emit the externally tagged `{"Unknown":"..."}` for the payload variant instead
of the plain wire string), and so must the `utoipa` schema (`PartialSchema` delegating to
`String`); `TimeInForce` follows it. An enum whose serde form is the derived variant name
(`Rule80A`, `OrderOpenClose`) keeps its derives: existing variants serialize as before and
`Unknown` takes the tagged form. There is deliberately no decode-time log when
`Unknown` is constructed — callers own the signal, as migration §9's example shows.
`OrderStatusKind` is the precedent (the official C# client's `OrderStatus` has the same
`Unknown` fallback). The criterion for opening another enum is its decode path, not its
vocabulary: inbound stream-parsed enums are candidates when the question arises;
one-shot and outbound-only enums stay closed — a hard error there fails one call, not
a stream.

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

## Integer-coded enums

A field that arrives as an `i32` code (`Liquidity`, `OcaType`, `TriggerMethod`, ...) has no
missing/empty half: the proto default is a real code. The rule for the value half is the same
as for strings: **`From<i32>` never coerces an unrecognized code to a known variant.** The
shape `Liquidity` established is a `#[repr(i32)]` enum with an `Unknown(i32)` payload
variant and a total `From<i32>` returning `Unknown(code)` for anything outside the table, so
the value is observable rather than silently read as `None`, `Default`, or the first listed
variant. The seven order enums also implement `From<T> for i32`, handing the code back,
written by hand because `value as i32` no longer compiles once a payload variant exists.
`Unknown(i32)` is `Copy`, so an enum that was `Copy` stays so, and derived serde stays.
There is no closed (`TryFrom<i32>`) form of this shape: these fields are decoded on a
streaming path where an error kills the subscription, so a payload variant is the only way
to keep the conversion total without coercing. `AuctionStrategy` takes the same shape for
uniformity only - the proto `Order` carries no such field
(`grep -rn auction_strategy src/proto/` is empty), so its `From<i32>` converts nothing but
the `i32` a caller hands `OrderBuilder`.

## Required tests

- Both `None` and `Some("")` produce `Err(Error::Parse(..))` for required fields, or
  `Ok(None)` for optional ones.
- A `Display` / `FromStr` round-trip over the full variant table.
- For an open enum: an unrecognized string parses as `Unknown(raw)` and round-trips
  through `Display` unchanged; empty still errors; a streaming subscription survives a
  frame carrying the unknown value (see `order_update_stream_survives_unknown_status`).
- For an integer-coded enum: `check_wire_code_round_trip` over the full code table plus
  `Unknown(code)` rows, and a decoder test feeding a code outside the table (see
  `decode_order_preserves_unknown_integer_codes`).

## Precedents

`docs/migration-3.0.md` §5, §9–§12, §23, §29, §30 is the shipped-outcome ledger for these
migrations — consult it rather than re-deriving which fields were converted and why.

- #518 — established the pattern (`OrderStatus.status` → `OrderStatusKind`).
- #556, #558 — established the generic `parse_required` / `parse_optional` shape.
- #559 — the `right: "?"` fixture-vs-wire lesson.
- #647 — `FundamentalReportType`: docs listed six values, TWS accepts four.
- #774 — the closed-enum counter-example: one unrecognized `OrderStatus` string
  terminated every order stream, including `order_update_stream`. Opened
  `OrderStatusKind` with `Unknown(String)` via the macro's `fallback` form; the
  missing/empty half of the directive is unchanged.
- #822 — `TimeInForce`: the failure without an open enum, on an infallible `From<&str>`
  rather than a `FromStr`. `GTX` had no variant, so `_ => Day` reported every GTX order
  as a day order and sent every GTX order as one — no parse error, nothing to observe.
  Opened it with `impl_wire_enum!(TimeInForce, fallback Unknown)`. Two notes for the
  next one: absent/empty here collapses to the struct default rather than `Error::Parse`,
  because upstream omits the field instead of sending it (`EClientUtils` / `EDecoderUtils`)
  — the directive's missing-input half assumes a field the wire always carries; and the
  variant table needs its own guard, since an enum this wide is tested from a hand-written
  table that a new variant does not break (`orders::tests::all_tifs_covers_every_variant`).
- #825 — the rest of the `src/orders` sweep, on top of #822. Three inherent `from(&str)`
  methods (`Action` panicked with `todo!()`, `Rule80A` / `OrderOpenClose` returned `None`)
  moved onto `impl_wire_enum!`; seven `From<i32>` impls that collapsed an unknown code to
  a known variant took the `Liquidity` shape. Per-enum call: `Action` closed (fixed
  vocabulary, `Copy` relied on by value); `Rule80A` and `OrderOpenClose` open (optional
  fields, already not `Copy`, so preserving costs nothing). `decode_order` moved onto
  `parse_required` / `parse_optional` for those three, so a missing `action` is now
  `Error::Parse` rather than `Buy` — unlike `tif` above, `action` has no unset state
  upstream, so an absent one is a malformed frame, not a default. `From<i32> for
  OrderCondition` panics on an unsupported discriminator and `decode_order_condition`
  falls back to a default price condition; both were left alone because they type a
  condition discriminator, not a field, and are a separate change.

