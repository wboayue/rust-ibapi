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
precedents: ["#518", "#556", "#558", "#559", "#647", "#774", "#822", "#825", "#829", "#832", "#827", "#855", "#1070"]
memory: [feedback_verify_wire_before_typing, feedback_helper_signature_precursor_pr, feedback_test_fixture_display_cruft, feedback_live_diagnostic_tests]
---

Before typing a `String` field as an enum, **verify the wire actually carries enumerated
values** — grep captured-wire fixtures and check the C# reference at
`/Users/wboayue/projects/tws-api/source/csharpclient/client/`. Field-name resemblance to a known
vocabulary is misleading.

Once verified: strict enum, `Display` round-trips to the IB wire string, `FromStr` returns
`Result<_, Error>`. For shape-identical enums, `impl_wire_enum!` (`src/macros.rs`) generates
`Display`, `FromStr<Err = Error>` and `ToField` from an `as_str` + `from_wire` table. Decode with
the generic helpers in `src/proto/decoders.rs` — no per-field wrapper:

```rust
parse_required::<OrderStatusKind>(proto.status.as_deref(), "status")?   // -> Result<T, Error>
parse_optional::<OptionRight>(proto.right.as_deref())?                  // -> Result<Option<T>, Error>
```

**Missing input is `Error::Parse`, never `T::default()`** — for a field the wire always carries.
Where the reference client treats an absent field as unset and still delivers the frame (`tif`,
#822: `EDecoderUtils` assigns it only if `HasTif`; `sec_type`, #855: `decodeContract` only if
`HasSecType`), absent or empty decodes to the field's unset value (`Order::default().tif`,
`SecurityType::Other("")`); only a direct `"".parse()` is `Error::Parse`.

**An unrecognized value on a streaming path parses as `Unknown(String)`.** A parse failure there
terminates the subscription, so a value IBKR adds later must not become an outage: use
`impl_wire_enum!(Name, fallback Unknown)`. Empty still errors. Matching stays exact and
case-sensitive (a case variant lands in `Unknown`, never coerced). The enum stays exhaustive so
the compiler points callers at the new arm; the payload variant costs `Copy` and makes `as_str`
return `&str`. No decode-time log — callers own the signal (`docs/migration-4.0.md` §9).
One-shot and outbound-only enums stay closed: a hard error there fails one call, not a stream.

Serde: if the enum serializes as the wire string (`OrderStatusKind`, `TimeInForce`), serde and the
`utoipa` schema (`PartialSchema` delegating to `String`) must be hand-written, or the derive emits
`{"Unknown":"..."}`. If it serializes as the derived variant name (`Rule80A`, `OrderOpenClose`),
keep the derives; `Unknown` takes the tagged form.

## Why

A silent `T::default()` masks an incomplete TWS response: the field reads as plausible, the
monitoring loop sees no error, and the subscription waits for state that already arrived malformed.

The wire is not what field names suggest. `OrderState.completed_status` sounds enumerated; it
carries free-form text like `"Cancelled by Trader"`. Typing strictly from the name produces a
decoder that rejects valid data.

When a new strict decoder rejects an existing test fixture, **don't broaden `FromStr` to accept
the literal.** Check `/Users/wboayue/projects/tws-api/samples/` first: #559's `right: "?"` was a
VB sample app's display fallback, never real wire.

## Integer-coded enums

A field arriving as an `i32` code (`Liquidity`, `OcaType`, `TriggerMethod`, ...) has no
missing/empty half; the proto default is a real code. **`From<i32>` never coerces an unrecognized
code to a known variant**: add an `Unknown(i32)` variant and make `From<i32>` total, returning
`Unknown(code)` outside the table. The order enums also implement `From<T> for i32`, hand-written
because `value as i32` no longer compiles with a payload variant. `Unknown(i32)` keeps `Copy` and
derived serde. There is no closed `TryFrom<i32>` form for streaming paths.

**A one-shot integer field decodes closed.** `ArticleType` (`news_article`, #1070) maps the table
in a private decoder fn and returns `Error::Parse` for any other code. Its public `From<i32>` still
coerces until a major release (#1071); an existing `From<i32>` also blocks `TryFrom<i32>` through
the blanket impl.

**The wire code lives in the `From` impls, not a discriminant.** No `#[repr(i32)]`, no `Foo = 0`:
with a payload variant nothing reads the discriminant, so it is decoration a reader takes for the
source of truth, and `#[repr(i32)]` is an ABI commitment buying nothing. A missing match arm is a
compile error; `Foo = 5` paired with `Foo => 6` is not. Document the code on each variant
(``Wire code `0`.``).

**Check `Order.proto` for the field before giving it this shape.** `AuctionStrategy` was typed "for
uniformity" in #825 on a field the proto `Order` does not carry (IBKR's `Order.proto` has no
`auctionStrategy`; `EClientUtils.createOrderProto` never sets one; `grep -rn auction_strategy
src/proto/` finds nothing). It was unreachable both ways, and was deleted with the `Order` field and
`auction_limit`'s fourth parameter (#828). Uniformity with a sibling enum is not evidence a field
exists.

## Required tests

- Both `None` and `Some("")` produce `Err(Error::Parse(..))` for required fields, or `Ok(None)` for
  optional ones.
- A `Display` / `FromStr` round-trip over the full variant table, plus a `modeled_index` guard (an
  exhaustive match, so a new variant without a table row fails to compile — see
  `all_tifs_covers_every_variant`).
- Open enum: an unrecognized string parses as `Unknown(raw)` and round-trips unchanged; empty still
  errors; a stream survives a frame carrying it (`order_update_stream_survives_unknown_status`).
- Integer-coded enum: `check_wire_code_round_trip` over the code table plus `Unknown(code)` rows
  (it probes every unlisted code in `-8..=64` and asserts each lands on `Unknown`), and a decoder
  test with an out-of-table code (`decode_order_preserves_unknown_integer_codes`).

## Precedents

Shipped-outcome ledger: `docs/migration-3.0.md` §5, §9–§12, §23, §29, §30;
`docs/migration-4.0.md` §2, §9, §13–§15; `docs/migration-5.0.md` §1, §7. Consult it rather than
re-deriving which fields were converted.

- #518 — established the pattern (`OrderStatus.status` → `OrderStatusKind`).
- #556, #558 — the generic `parse_required` / `parse_optional` shape.
- #559 — the `right: "?"` fixture-vs-wire lesson.
- #647 — `FundamentalReportType`: docs listed six values, TWS accepted four (module since removed,
  migration-3.0 §34).
- #774 — one unrecognized `OrderStatus` string terminated every order stream; opened
  `OrderStatusKind` with `Unknown(String)` via the macro's `fallback` form.
- #822 — `TimeInForce`: `_ => Day` on an infallible `From<&str>` reported and sent every GTX order
  as a day order, with nothing to observe. Opened via `fallback Unknown`; absent/empty collapses to
  the struct default because upstream omits the field.
- #825 — the rest of `src/orders`: inherent `from(&str)` methods moved onto `impl_wire_enum!`
  (`Action` closed; `Rule80A`, `OrderOpenClose` open); seven coercing `From<i32>` impls took the
  `Liquidity` shape. A missing `action` became `Error::Parse` rather than `Buy` — unlike `tif`, it
  has no unset state upstream.
- #829 — review follow-up and counter-example: dropped #825's `#[repr(i32)]` + discriminants
  (copied faithfully from `Liquidity`, still wrong); found `AuctionStrategy`'s claimed caller had no
  setter; made `_every_wire_code` test names true with the `Unknown`-constructor probe. Copying a
  precedent is not checking it still earns its shape.
- #827 — `OrderCondition`: an unmodeled type decoded as `Price(PriceCondition::default())`, so
  re-placing the order sent TWS a different condition. When a discriminator selects a *message
  shape*, `Unknown(code)` is not enough: `Unknown(UnknownCondition)` carries the code plus every wire
  field as `Option`, so encode(decode(p)) == p except an absent conjunction flag (returns as
  `Some(true)`). Absent `type` is `Error::Parse`. The panicking `From<i32> for OrderCondition` was
  removed.
- #855 — `SecurityType` onto `impl_wire_enum!(SecurityType, fallback Other)`. `sec_type` is the
  #822-style exception: requiring it would end every stream carrying a contract without one (the
  failure #840 fixed). The `ContractData` decoder takes `required` (`src/proto/decoders.rs`) for
  both submessages, as `EDecoder.cs` drops such a frame.
