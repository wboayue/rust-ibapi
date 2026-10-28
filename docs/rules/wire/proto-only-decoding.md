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
symbols: [require_proto, ResponseMessage::proto, StreamDecoder, TickDecoder, RESPONSE_MESSAGE_IDS, expect_type, Error::UnexpectedResponse, Error::UnexpectedWireFormat]
related: [proto-aware-accessors, enum-typing, fixture-builders, one-shot-narrowing]
precedents: ["#508", "#731", "#732", "#733", "#734", "#735", "#738", "#739", "#740"]
memory: [project_protobuf_only, feedback_unreachable_regression_guards]
---

Every domain decoder reads its payload with `message.proto()`, which takes the bytes from
`require_proto()` and decodes them through `DecodeProto::decode_proto` (prost's error becomes
`Error::ProtobufDecode`). No text branch, no format dispatch.

```rust
pub(in crate::news) fn decode_news_bulletin(message: &ResponseMessage) -> Result<NewsBulletin, Error> {
    decode_news_bulletin_proto(message.proto()?)
}
```

**Decoding takes `&ResponseMessage`, never `&mut`.** `next_int` / `next_string` still need `&mut`
(`next_double` is `#[cfg(test)]`), but their only production caller is the handshake
(`connection/common.rs`), which reads a frame that predates any decoder. A new `&mut` in a decode
path is a text decoder or a signature copied from one.

**`RESPONSE_MESSAGE_IDS` must list every type the `decode` match handles.** It is the skip
filter: all three drivers (`StreamDecoder` sync and async, and
`market_data/historical/common/tick.rs::classify` for `TickDecoder`) drop anything not listed,
via `is_undeclared`, *before* calling `decode`. An arm for an unlisted type is dead code. The
const stays a slice even for single-type decoders.

**Every `decode` needs a backstop.** A multi-type match ends with
`_ => Err(Error::unexpected_response(message))`. A single-type decoder has no match, so
`message.expect_type(IncomingMessages::X)?` is its backstop — not redundant with the const (the
scanner, `WshEventData`, and the three `TickDecoder` impls take this form). Hardcode that literal;
don't derive it from `Self::RESPONSE_MESSAGE_IDS`, or the gate below compares the const with
itself. The backstop terminates the subscription loudly. Never `Error::NotImplemented` or
`Error::Simple(...)`.

**Never declare `IncomingMessages::Error` or `Shutdown`, and never write an arm for them.**
`determine_routing` classifies both before any allow-list: an error reaches the subscription as
`RoutedItem::Error`/`Notice`, a shutdown ends the dispatcher loop. One-shots read through
`RoutedItem::into_legacy`, where an error becomes `Some(Err(_))` and the processor never runs.

## The gates

`response_message_ids_match_decode_arms` (`src/subscriptions/response_message_ids_tests.rs`)
probes every decoder with a minimal text-framed message of every `IncomingMessages` discriminant
and requires declared and handled to agree exactly, for both traits: `UnexpectedResponse` means
"no arm"; anything else (`Ok`, `EndOfStream`, or `UnexpectedWireFormat` from `proto()` on text
framing) means an arm exists. It also rejects declaring a dispatcher-intercepted type. Handled
but undeclared would otherwise be silent at runtime.

That reading is why a missing backstop is not a style question: a `decode` that dives straight
into `proto()` answers `UnexpectedWireFormat` to every probe and reads as handling everything.

`decoder_roster_is_complete` counts `impl StreamDecoder` and `impl TickDecoder` blocks under
`src/` against the hand-listed roster, per trait, and fails on an unregistered decoder.

## Why

Skip used to be keyed on an error variant: `UnexpectedResponse` meant "drop this", while ~20
one-shot sites returned the same variant to users as a genuine error. That is the defect behind
#508 (a subscription died on one unrecognized message) and #731 (a mis-framed fixture vanished).
#732 moved the decision to the declared list, where it is data rather than an error code.

The two errors look alike and are not:

| Call site | Meaning |
|---|---|
| `_ => Err(Error::unexpected_response(message))` | declared but unhandled — a bug in this impl |
| `message.proto()?` → `UnexpectedWireFormat` | handled, but the framing is unreadable |

Both terminate; neither is skipped. A mis-framed fixture fails its test instead of staying green
with its assertions unrun ([fixture builders](../testing/fixture-builders.md)). At
`server_versions::PROTOBUF_REST_MESSAGES_3` every message with a proto decoder arrives
proto-framed, so `UnexpectedWireFormat` in production means the gateway broke protocol.
`ResponseMessage::peek_int` returns the same variant for the mirror case (proto framing at a
text-field accessor).

`Error::UnexpectedResponse` carries a `String`, built by `Error::unexpected_response(&message)`,
because `ResponseMessage` is `pub(crate)` and cannot sit in a public enum payload.

## Related surfaces

Proto framing must be respected in three places; when fixing one, audit the others:

1. Decoders — this node.
2. `ResponseMessage` accessors — [proto-aware accessors](proto-aware-accessors.md).
3. Error frames — `routing::decode_error_envelope`, behind the dispatcher's `DecodedError` and
   `From<&ResponseMessage> for Notice`. Proto-only; absent or undecodable bytes yield an empty
   code-0 notice.

## Precedents

- #508 — an unknown message type terminated the subscription instead of being skipped.
- #731 — split the framing failure out of the skip path so the fixture trap fails loudly.
- #732 — retired skip-by-error-variant; `RESPONSE_MESSAGE_IDS` became the filter, with no default.
- #733 — gated both drift directions; roster counted against the tree.
- #734 — audited all 78 declared entries; removed sixteen `IncomingMessages::Error` declarations
  and their arms, plus the guard exemption that kept them legal.
- #735 — same removal on the one-shot side; surfaced blocking `matching_symbols` discarding
  routed errors and returning `Ok(vec![])`.
- #738 — extended the const to `TickDecoder` (one shared skip filter). Counter-example: it first
  deleted the `expect_type` from two single-type `StreamDecoder` impls as duplication, and the
  gate caught it within the hour.
- #739 — gated `TickDecoder`, which first required adding `expect_type` backstops to all three
  impls; without them every probe read as handled. The over-declaration #738 cited (`TickBidAsk`
  declaring `UserInfo`) passed before and fails by name after.
- #740 — narrowed `decode` from `&mut` to `&ResponseMessage` across 29 impls; no production
  decoder mutated. Find real cursor users with
  `grep -rn "\.next_int()\|\.next_string()\|\.next_double()" --include=*.rs src/ | grep -v "tests\.rs"` (only `connection/common.rs`).
