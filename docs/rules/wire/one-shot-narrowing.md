---
id: one-shot-narrowing
title: One-shot requests narrow through expect_proto
cluster: wire
status: active
triggers:
  - adding a one-shot client method
  - passing a processor to one_shot_shared or one_shot_by_request_id
  - wondering whether a one-shot should retry
  - adding a decode_*_proto sibling for a one-shot response
  - adding a one-shot whose request TWS can cancel
symbols: [expect_proto, ProtoPayload, one_shot_shared, one_shot_shared_matching, ONE_SHOT_TIMEOUT, one_shot_deadline, check_deadline, next_frame, one_shot_by_request_id, one_shot_cancelling, CancellableOneShot, fold_one_shot, empty_on_end_of_stream, expect_type, retry_on_connection_reset]
related: [proto-only-decoding, proto-aware-accessors, fixture-builders]
precedents: ["#736", "#738", "#740", "#741", "#745", "#749", "#1107"]
memory: [project_protobuf_only, feedback_request_id_index_registration]
---

A one-shot request reads one frame off a **shared** channel, so it must check that the frame is
the one it asked for. Do that with `request_helpers::expect_proto`, never a bare decoder:

```rust
request_helpers::blocking::one_shot_by_request_id(
    self,
    encoders::encode_request_user_info,
    expect_proto(decoders::decode_user_info_proto),
)
```

The expected message type is not written here: it is `ProtoPayload::MESSAGE_ID` on the payload
the decoder accepts (`decode_user_info_proto(p: proto::UserInfo)`), so naming the decoder names
the frame. A one-shot decoder therefore takes the decoded `prost` type, not `&[u8]`; give the
payload an `impl ProtoPayload` (`src/proto/payload.rs`) if it lacks one — `expect_proto` won't
compile without it.

**A one-shot TWS can cancel goes through `one_shot_cancelling::<K>`**, where `K` is a
`CancellableOneShot` naming the payload type, its decoder and its cancel encoder. The request is
read through a typed subscription, so one dropped before its answer (dropped future, timeout,
local error) is cancelled at TWS; the answer or a TWS error ends the request without a cancel.
When adding a one-shot, check C# `EClient.cs` for a `cancel*` method: head timestamp, histogram,
historical schedule, option price, implied volatility and both WSH requests have one.

**Every one-shot helper retries.** `one_shot_by_request_id` for a request-id request,
`one_shot_shared` (or `one_shot_shared_matching`) for a shared channel. None takes a
`ProtocolFeature`: the handshake requires server version 213, so most one-shots need no check; one
gated above the floor checks inline first, as `config` does with
`check_version(self.server_version(), Features::CONFIG)?` (#978). `fold_one_shot` is private to
`request_helpers` so a non-retrying hand-rolled one-shot is unreachable from a domain module.

**No helper takes an `on_none`**: a closed stream is `Error::UnexpectedEndOfStream`. Where "TWS
sent nothing" is a legitimate empty answer, chain `.or_else(empty_on_end_of_stream)`.

**Deadlines.** A shared one-shot waits at most `ONE_SHOT_TIMEOUT`, then fails with
`Error::Timeout` (the frame carries no request id, so nothing else ends the wait). A request-id
one-shot waits without a limit. `ClientBuilder::one_shot_timeout` replaces both bounds. The
deadline is taken once, outside `retry_on_connection_reset`, so it bounds the whole call; each
attempt calls `check_deadline` before sending, so a reconnect that outlasts it sends nothing. A
helper that reads more than once (historical-data bars, then its window) takes the deadline from
`one_shot_deadline` and passes it to every `next_frame`.

**Concurrent callers.** Every live subscription of a type receives every frame of that type, so a
shared one-shot whose frames can belong to another caller uses `one_shot_shared_matching`: its
processor answers `Ok(None)` for a foreign frame and reads on. `request_fa` (one `ReceiveFA` type
for every dataset) and `market_rule` (one `MarketRule` type for every rule id) use it.

Don't use `expect_proto` inside `impl StreamDecoder::decode`; see
[proto-only decoding](proto-only-decoding.md) for what that surface owes instead.

## Why

Narrowing used to be opt-in: each domain hand-wrote a `decode_*_message` wrapper doing
`expect_type(..)?`, and 26 of 50 sites had none — they decoded whatever arrived. The follow-up
scoping #738 estimated four; nobody had run a command for the claim. The failure is quiet: a
foreign proto decoded as the wrong `prost` type usually yields *something* (field numbers overlap),
so the caller gets a plausible struct of wrong values.

Where the `(message type, decoder)` pair lives is the point. #738 put both at the call site,
which made narrowing structural but left the two arguments unrelated to the compiler, so a
mispairing built and ran; catching it took a hand-listed roster and a source-scraping test. #749
moved the type onto the payload, where it is a property rather than an argument. A wrong
`MESSAGE_ID` in an impl is still possible; the per-API round-trip tests catch it with `expected
FamilyCodes, got .. kind: UserInfo` instead of asserting on mis-decoded bytes.

`expect_proto` is a combinator returning `impl Fn(&ResponseMessage)`, not another helper
parameter, keeping the helper signatures at three and four arguments.

**Why every one-shot retries:** `retry_on_connection_reset` fires only on
`Error::ConnectionReset`, up to `DEFAULT_MAX_RETRIES` (3). Every one-shot is a read, so replaying
it after a dropped connection costs nothing and loses no server-side state. The removed
`one_shot_request` was the only helper taking a `ProtocolFeature`, so version-gated sites picked
it and silently lost retry — the distinction was never about retry. When two helpers differ on two
axes and callers only care about one, the other axis gets chosen for them.

## Precedents

- #736 — collapsed seven `decode_*_message` dispatchers onto `ResponseMessage::expect_type`, but
  left narrowing per-domain, so sites without a wrapper stayed unnarrowed.
- #738 — moved the pair to the call site, deleted 30 wrappers, narrowed the last site
  (`next_valid_order_id` on the shared `RequestIds` channel).
- #740 — `StreamDecoder::decode` took `&ResponseMessage`, letting the four option-computation
  sites drop hand-rolled `send_raw` + fold for the retrying helper.
- #741 — deleted `one_shot_request` and `fold_one_shot_mut`; every one-shot retries.
- #745 — dropped `on_none`: 44 of 54 sites passed the same closure, the other 10 one empty vector
  spelled two ways.
- #749 — `ProtoPayload` moved the pair onto the payload; roster and scraping test deleted. Keying
  on the decoded type fails (`server_time`, `server_time_millis`, `head_timestamp` all return
  `OffsetDateTime`); keying on the `prost` payload has no collisions.
- #1107 — the seven one-shots with an IB cancel moved to `one_shot_cancelling`; before, a dropped
  or timed-out request kept running at TWS. Live: each is answered by exactly one frame. A frame of
  another type is now skipped by the typed subscription's `RESPONSE_MESSAGE_IDS` filter rather
  than reported as `UnexpectedResponse`.
