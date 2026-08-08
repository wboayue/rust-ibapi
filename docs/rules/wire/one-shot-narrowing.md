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
symbols: [expect_proto, ProtoPayload, one_shot_shared, one_shot_by_request_id, fold_one_shot, empty_on_end_of_stream, expect_type, retry_on_connection_reset]
related: [proto-only-decoding, proto-aware-accessors, fixture-builders]
precedents: ["#736", "#738", "#740", "#741", "#745", "#749"]
memory: [project_protobuf_only, feedback_request_id_index_registration]
---

A one-shot request reads one frame off a **shared** channel, so it must check that the frame is
the one it asked for. Do that with `request_helpers::expect_proto`, never with a bare decoder:

```rust
request_helpers::blocking::one_shot_by_request_id(
    self,
    encoders::encode_request_user_info,
    expect_proto(decoders::decode_user_info_proto),
)
```

The expected message type is not written here. It is `ProtoPayload::MESSAGE_ID` on the payload
the decoder accepts — `decode_user_info_proto(p: proto::UserInfo)` — so naming the decoder names
the frame. A one-shot API's decoder therefore takes the *decoded* `prost` type, not `&[u8]`;
give the payload an `impl ProtoPayload` (`src/proto/payload.rs`) if it lacks one, which
`expect_proto` will demand anyway since it cannot infer `MESSAGE_ID` otherwise.

**There are two one-shot helpers, and both retry.** `one_shot_by_request_id` for a
request-id request, `one_shot_shared` for a shared channel. Neither takes a
`ProtocolFeature` — do the version check on the line above. Usually that is
`check_version(server_version, Features::X)?`, but not always: `news` uses
`check_server_version(..)` and `historical_data` uses `validate_historical_data(..)`, which is
half of why a `feature` parameter cannot come back. 44 of the 54 one-shot sites check a version;
the 10 that do not are `server_time`, `managed_accounts`, `request_fa`, `next_valid_order_id`,
and `scanner_parameters`, none of which carries a `MinServerVer` in the C# client.

Neither helper takes an `on_none` either: a closed stream is `Error::UnexpectedEndOfStream`.
The ten sites where "TWS sent nothing" is a legitimate empty answer chain
`.or_else(empty_on_end_of_stream)`.

A one-shot that does not retry is a bug, not a choice. `fold_one_shot` is private to
`request_helpers` so that hand-rolling one is not reachable from a domain module.

Do not reach for `expect_proto` inside `impl StreamDecoder::decode`; see
[proto-only decoding](proto-only-decoding.md) for what that surface owes instead.

## Why

Narrowing used to be opt-in. Each domain hand-wrote a `decode_*_message` wrapper doing
`expect_type(..)?` before its real decoder, and 26 of the 50 `expect_proto` sites had no wrapper
at all — they decoded whatever arrived on the channel. The follow-up that scoped #738 put that
number at four; the real one was six times larger, which is the usual shape of a
completeness claim nobody ran a command for.

The consequence is quiet. A foreign proto handed to the wrong `prost` type usually decodes to
*something* — field numbers overlap across messages — so the caller gets a plausible struct full
of wrong values rather than an error.

**The pair moved twice, and where it landed is the point.** #738 put it at the call site as
`expect_proto(IncomingMessages::UserInfo, decode_user_info_proto)`, which made narrowing
structural — a site could not name a payload decoder without naming a frame — but left the two
arguments unrelated to the compiler, so a mispairing built and ran. Catching that took a
hand-listed roster and a source-scraping test, and the pair was spelled twice per API since sync
and async each carry a call site. #749 moved it onto the payload type, where it is a property
rather than an argument: 25 `impl ProtoPayload` declarations replace 50 call-site literals plus
the roster.

That is not the same as "the type system enforces it". A wrong `MESSAGE_ID` in an impl is still
possible; it is caught by the per-API round-trip tests, which now fail with `expected
FamilyCodes, got .. kind: UserInfo` rather than decoding one message's bytes into another's type
and asserting on the result.

`expect_proto` is a combinator returning `impl Fn(&ResponseMessage)` rather than another
parameter on the two one-shot helpers, which take three and four arguments against a budget of
three (see [param budget](../style/param-budget.md)) with no builder in front of them.

## Why every one-shot retries

`retry_on_connection_reset` fires only on `Error::ConnectionReset` and gives up after three
attempts. Every one-shot here is a read — encode, send, read one frame — so replaying it after
the gateway drops the connection costs nothing and loses no server-side state. There is no
one-shot for which the answer differs, which is why the helper no longer offers the choice.

It used to. A third helper, `one_shot_request`, was the only one taking a `ProtocolFeature`, so
the four version-gated shared-channel one-shots (`market_rule`, `family_codes`, sync and async)
picked it and silently gave up retry — while `market_depth_exchanges`, equally version-gated,
called `check_version` inline and kept it. **The distinction was never about retry.** #741
deleted the helper and moved its four sites to the inline form, so the two survivors differ only
in whether the request carries an id.

The general shape: when two helpers differ on two axes and callers only care about one, the
axis they are not choosing gets chosen for them.

## Precedents

- #736 — collapsed seven `decode_*_message` dispatchers onto `ResponseMessage::expect_type`.
  Right direction, but it left narrowing as a per-domain wrapper, so the sites without one
  stayed unnarrowed and invisible.
- #738 — moved the pair to the call site, deleted 30 wrapper functions, and swept the last
  unnarrowed site (`next_valid_order_id`, on the shared `RequestIds` channel). The gate came
  second, after a review mutated three sites and found only incidental round-trip tests failed;
  a roster test then failed the same mutation by name.
- #740 — narrowed `StreamDecoder::decode` to `&ResponseMessage`, which is what let the four
  option-computation sites stop hand-rolling `send_raw` + fold. Their decoder wanted `&mut`, and
  the retrying helper's processor bound is `Fn(&ResponseMessage)`; the `&mut` turned out to be
  vestigial rather than a real constraint.
- #741 — deleted `one_shot_request` and `fold_one_shot_mut`; every one-shot retries and no
  hand-rolled folds remain. Mechanism above.
- #745 — dropped `on_none`. 44 of 54 sites passed the same closure and the other 10 passed one
  empty vector spelled two ways, which is the tell that nobody was choosing.
- #749 — `ProtoPayload` moved the pair onto the payload; the roster and its scraping test are
  deleted. The plan that proposed this keyed the trait on the **decoded** type, which cannot
  work: `server_time`, `server_time_millis`, and `head_timestamp` all return `OffsetDateTime`,
  so one type would have needed three `MESSAGE_ID`s. Keying on the `prost` payload has no
  collisions — a counter-example worth keeping, since the version that fails is the one that
  reads more natural.
