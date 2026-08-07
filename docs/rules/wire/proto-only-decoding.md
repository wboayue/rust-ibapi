---
id: proto-only-decoding
title: Domain decoders are protobuf-only
cluster: wire
status: active
triggers:
  - writing or modifying a domain decoder
  - adding a StreamDecoder impl
  - a subscription terminates on an unexpected message
symbols: [require_proto, process_decode_result, StreamDecoder, Error::UnexpectedResponse]
related: [proto-aware-accessors, enum-typing, fixture-builders]
precedents: ["#508"]
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

`require_proto()` returns `Error::UnexpectedResponse` for a text-framed message, so a stale
test fixture or a future-version regression degrades to a skip rather than a panic or a
dead subscription. Note the cost of that safety: a text-framed fixture pointed at a
proto-only decoder produces a **passing test whose post-`next_data()` assertions never run**.
See [fixture builders](../testing/fixture-builders.md).

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
