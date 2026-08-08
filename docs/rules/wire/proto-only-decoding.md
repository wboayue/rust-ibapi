---
id: proto-only-decoding
title: Domain decoders are protobuf-only
cluster: wire
status: active
triggers:
  - writing or modifying a domain decoder
  - adding a StreamDecoder impl
  - a subscription terminates on an unexpected message
  - adding a decode arm for a new message type
symbols: [require_proto, process_decode_result, StreamDecoder, RESPONSE_MESSAGE_IDS, Error::UnexpectedResponse, Error::UnexpectedWireFormat]
related: [proto-aware-accessors, enum-typing, fixture-builders]
precedents: ["#508", "#731", "#732", "#733", "#734"]
memory: [project_protobuf_only, feedback_unreachable_regression_guards]
---

Every domain decoder reads its payload with `message.require_proto()` and feeds the bytes to
`prost::Message::decode(...)`. There is no text branch and no format dispatch — `decode_proto_or_text`
was retired with the floor ratchet.

**`RESPONSE_MESSAGE_IDS` must list every type the `decode` match handles.** It is the skip
filter: the sync and async subscription drivers drop anything not listed there *before* calling
`decode`, because shared channels carry several types. A `decode` arm for an unlisted type is
dead code. (The historical-tick driver in `market_data/historical/common/tick.rs` has its own
single-valued `TickDecoder::MESSAGE_TYPE` doing the same job — a third mechanism, not yet
unified.)

End every `impl StreamDecoder<T>::decode` match with `_ => Err(Error::unexpected_response(message))`
anyway. It is now a backstop for the two lists disagreeing, not a control-flow signal — it
terminates the subscription, loudly. Never `Error::NotImplemented` or `Error::Simple(...)`.

**Both drift directions are gated**, by `test_response_message_ids_match_decode_arms`
(`src/subscriptions/response_message_ids_tests.rs`). It probes every decoder with a minimal
text-framed message of every `IncomingMessages` discriminant and requires the two lists to
agree exactly: `UnexpectedResponse` means "no arm" (nothing else reaches the backstop),
anything else means an arm exists — `Ok`, `EndOfStream`, or the `UnexpectedWireFormat` that
`require_proto()` raises on text framing. Declared-but-unhandled would also fail loudly at
runtime via the backstop; handled-but-undeclared would not, which is why the test exists.

Its companion `test_decoder_roster_is_complete` counts `impl StreamDecoder` blocks under `src/`
against the hand-listed roster, so adding a decoder without registering it fails rather than
going unchecked.

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
