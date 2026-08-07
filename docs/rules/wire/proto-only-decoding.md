---
id: proto-only-decoding
title: Domain decoders are protobuf-only
cluster: wire
status: active
triggers:
  - writing or modifying a domain decoder
  - adding a StreamDecoder impl
  - a subscription terminates on an unexpected message
symbols: [require_proto, process_decode_result, StreamDecoder, Error::UnexpectedResponse, Error::UnexpectedWireFormat]
related: [proto-aware-accessors, enum-typing, fixture-builders]
precedents: ["#508", "#731"]
memory: [project_protobuf_only, feedback_unreachable_regression_guards]
---

Every domain decoder reads its payload with `message.require_proto()` and feeds the bytes to
`prost::Message::decode(...)`. There is no text branch and no format dispatch — `decode_proto_or_text`
was retired with the floor ratchet.

End every `impl StreamDecoder<T>::decode` match with `_ => Err(Error::unexpected_response(message))`.
Never `Error::NotImplemented` or `Error::Simple(...)`.

```rust
pub(in crate::news) fn decode_news_bulletin(message: &ResponseMessage) -> Result<NewsBulletin, Error> {
    decode_news_bulletin_proto(message.require_proto()?)
}
```

## Why

The catch-all arm decides what an unrecognised message does to a live subscription.
`process_decode_result` (`src/subscriptions/common.rs`) maps `UnexpectedResponse` to
`ProcessingResult::Skip` — the message is dropped and the subscription survives. Every other
error variant terminates it. A subscription that dies because TWS sent one message the
decoder didn't recognise is the bug class of issue #508.

`require_proto()` returns a **different** variant — `Error::UnexpectedWireFormat` — and that
one is *not* skippable. The two failures look alike and are not:

| Call site | Meaning | Disposition |
|---|---|---|
| `_ => Err(Error::unexpected_response(message))` | not my message type | `Skip` — shared channels carry several types |
| `message.require_proto()?` | my message type, unreadable framing | `Error` — the message was addressed to this decoder |

A mis-framed fixture therefore fails its test rather than leaving it green with the
post-`next_data()` assertions unrun — see [fixture builders](../testing/fixture-builders.md).
At `server_versions::PROTOBUF_REST_MESSAGES_3` every message with a proto decoder arrives
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
