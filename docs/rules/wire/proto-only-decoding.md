---
id: proto-only-decoding
title: Domain decoders are protobuf-only
cluster: wire
status: active
triggers:
  - writing or modifying a domain decoder
  - adding a StreamDecoder or TickDecoder impl
  - a subscription terminates on an unexpected message
  - adding a decode arm for a new message type
symbols: [require_proto, process_decode_result, StreamDecoder, TickDecoder, RESPONSE_MESSAGE_IDS, expect_type, Error::UnexpectedResponse, Error::UnexpectedWireFormat]
related: [proto-aware-accessors, enum-typing, fixture-builders, one-shot-narrowing]
precedents: ["#508", "#731", "#732", "#733", "#734", "#735", "#738", "#739"]
memory: [project_protobuf_only, feedback_unreachable_regression_guards]
---

Every domain decoder reads its payload with `message.require_proto()` and feeds the bytes to
`prost::Message::decode(...)`. There is no text branch and no format dispatch — `decode_proto_or_text`
was retired with the floor ratchet.

**`RESPONSE_MESSAGE_IDS` must list every type the `decode` match handles.** It is the skip
filter: all three drivers drop anything not listed there *before* calling `decode`, because
shared channels carry several types. A `decode` arm for an unlisted type is dead code. The third
driver is `market_data/historical/common/tick.rs::classify`, over `TickDecoder`, which declares
the same const and filters with the same `is_undeclared` helper — a tick type simply declares
one entry (#738).

**Every `decode` needs a backstop, and it is what makes the gate below able to read it.** End a
multi-type `impl StreamDecoder<T>::decode` match with
`_ => Err(Error::unexpected_response(message))`. A decoder consuming exactly one type has no
match to hang that on, so `message.expect_type(IncomingMessages::X)?` is its backstop and is
*not* redundant with the const — that is the form the scanner, both wsh, and all three
`TickDecoder` impls take. Either way it is a backstop for the two lists disagreeing, not a
control-flow signal: it terminates the subscription, loudly. Never `Error::NotImplemented` or
`Error::Simple(...)`.

**Both drift directions are gated, for both traits**, by `test_response_message_ids_match_decode_arms`
(`src/subscriptions/response_message_ids_tests.rs`). It probes every decoder with a minimal
text-framed message of every `IncomingMessages` discriminant and requires the two lists to
agree exactly: `UnexpectedResponse` means "no arm" (nothing else reaches the backstop),
anything else means an arm exists — `Ok`, `EndOfStream`, or the `UnexpectedWireFormat` that
`require_proto()` raises on text framing. Declared-but-unhandled would also fail loudly at
runtime via the backstop; handled-but-undeclared would not, which is why the test exists.

That reading is exactly why a missing backstop is not a style question. A `decode` that dives
straight into `require_proto()` answers `UnexpectedWireFormat` to *every* probe, so the gate
reads it as claiming an arm for every discriminant it scans and can say nothing about it.

**The const stays a slice even for the six single-type decoders, and the narrow stays a
hardcoded literal.** Both look like duplication and neither is. Collapsing to
`const MESSAGE_TYPE` with a derived `RESPONSE_MESSAGE_IDS = &[Self::MESSAGE_TYPE]` compiles, but
`StreamDecoder`'s multi-type impls cannot use it, so one filter (`is_undeclared`) would read two
declaration forms. And deriving the narrow from `Self::RESPONSE_MESSAGE_IDS` makes the gate
circular — `handled` would be computed from the same const as `declared`, so every declaration
would agree with itself, including a wrong one. A cross-check needs two independent statements.

Its companion `test_decoder_roster_is_complete` counts `impl StreamDecoder` and `impl TickDecoder`
blocks under `src/` against the hand-listed roster — per trait, so adding one while deleting the
other cannot net out — and fails when a new decoder goes unregistered.

**Never declare `IncomingMessages::Error` or `Shutdown`, and never write a `decode` arm for
them.** `determine_routing` classifies both before any allow-list, so an error reaches the
subscription as `RoutedItem::Error`/`Notice` and a shutdown ends the dispatcher loop — neither
ever arrives as the `RoutedItem::Response` that `decode` consumes. The same holds for one-shot
requests, which read through `RoutedItem::into_legacy`: an error becomes `Some(Err(_))` and the
processor never runs. Sixteen decoders declared `Error` anyway; the check above names the
mistake specifically.

```rust
pub(in crate::news) fn decode_news_bulletin(message: &ResponseMessage) -> Result<NewsBulletin, Error> {
    decode_news_bulletin_proto(message.require_proto()?)
}
```

## Why

`process_decode_result` (`src/subscriptions/common.rs`) used to map `UnexpectedResponse` to
`ProcessingResult::Skip`, which made an error variant carry dispatch semantics. That is the
defect behind both #508 (a subscription died on one unrecognised message) and #731 (a
mis-framed fixture vanished): the same variant is returned to users as a genuine error by ~20
one-shot call sites, so any decoder that reused it silently inherited "drop this". #732 moved
the decision to the declared list, where it is data rather than an error code.

`require_proto()` returns `Error::UnexpectedWireFormat`, distinct from `UnexpectedResponse`
because the two failures look alike and are not:

| Call site | Meaning |
|---|---|
| `_ => Err(Error::unexpected_response(message))` | declared but unhandled — a bug in this impl |
| `message.require_proto()?` | handled, but the framing is unreadable |

Both terminate. Neither is skipped. A mis-framed fixture therefore fails its test rather than
leaving it green with the post-`next_data()` assertions unrun — see
[fixture builders](../testing/fixture-builders.md). At
`server_versions::PROTOBUF_REST_MESSAGES_3` every message with a proto decoder arrives
proto-framed, so `UnexpectedWireFormat` in production means the gateway broke protocol.

`ResponseMessage::peek_int` is the mirror image — proto framing at a text-field accessor —
and returns the same variant. A framing mismatch is never skippable in either direction.

`Error::UnexpectedResponse` carries a `String`, not a `ResponseMessage` — the constructor
`Error::unexpected_response(&message)` formats it, because `ResponseMessage` became
`pub(crate)` and could no longer sit in a public enum payload.

## Related surfaces

This is one of three places the proto framing must be respected. When fixing one, audit the
other two:

1. Decoders — this node.
2. `ResponseMessage` accessors — [proto-aware accessors](proto-aware-accessors.md).
3. The `From<&ResponseMessage> for Notice` / `From<ResponseMessage> for Error` conversions,
   which read `raw_bytes` first and fall back to text fields only for legacy fixtures.

## Precedents

- #508 — the original bug: an unknown message type terminated the subscription instead of
  being skipped.
- #731 — split the framing failure out of the skip path so the fixture trap fails loudly.
- #732 — retired skip-by-error-variant entirely; `RESPONSE_MESSAGE_IDS` is now the filter and
  the const lost its default so every impl must declare one.
- #733 — gated both drift directions; the roster is counted against the tree.
- #734 — audited all 78 declared entries under the const's new meaning. Sixteen declared
  `IncomingMessages::Error`, which the dispatcher intercepts; those and their `decode` arms are
  gone, along with the guard exemption that had been keeping them legal.
- #735 — same removal on the one-shot side, and `MessageBusStub` now classifies error frames
  so the input those arms handled can no longer be constructed. Surfaced a real bug behind one
  of them: blocking `matching_symbols` discarded routed errors and returned `Ok(vec![])`.
- #738 — extended the const to `TickDecoder` so all three drivers share one skip filter, and
  moved one-shot narrowing to `request_helpers::expect_proto` at the call site (gated separately
  by `test_expect_proto_sites_match_the_roster`). **A counter-example too:** the same PR first
  deleted the `expect_type` from the two single-type `StreamDecoder::decode` impls as a "third
  copy of the same fact", and `test_response_message_ids_match_decode_arms` caught it within the
  hour. The narrow is the single-arm form of the backstop, not a duplicate of the const. It also
  showed the const's rename to `TickDecoder` bought the name without the gate.
- #739 — closed that gap, and the ordering is the lesson. The gate could not simply be pointed at
  `TickDecoder`: with no backstop in the three impls, every probe reached `require_proto()` and
  the decoders read as handling everything. Adding `expect_type` first is what made them legible,
  which is the same fact #738 learned from the other direction — it deleted two backstops as
  redundant, this one had to add three before a gate could exist. Confirmed against the tree: the
  over-declaration #738 used as its proof (`TickBidAsk` declaring `UserInfo`) passed all 1364 sync
  tests before, and now fails by name.
