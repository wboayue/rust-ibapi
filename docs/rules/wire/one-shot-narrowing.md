---
id: one-shot-narrowing
title: One-shot requests narrow through expect_proto
cluster: wire
status: active
triggers:
  - adding a one-shot client method
  - passing a processor to one_shot_with_retry or one_shot_request_with_retry
  - wondering whether a one-shot should retry
  - adding a decode_*_proto sibling for a one-shot response
symbols: [expect_proto, one_shot_with_retry, one_shot_request_with_retry, fold_one_shot, expect_type, retry_on_connection_reset, PAIRS]
related: [proto-only-decoding, proto-aware-accessors, fixture-builders]
precedents: ["#736", "#738", "#740", "#741"]
memory: [project_protobuf_only, feedback_request_id_index_registration]
---

A one-shot request reads one frame off a **shared** channel, so it must check that the frame is
the one it asked for. Do that with `request_helpers::expect_proto`, never with a bare decoder:

```rust
request_helpers::blocking::one_shot_request_with_retry(
    self,
    encoders::encode_request_user_info,
    expect_proto(IncomingMessages::UserInfo, decoders::decode_user_info_proto),
    || Err(Error::UnexpectedEndOfStream),
)
```

The processor argument is the only place the expected type appears, so a new one-shot API cannot
narrow "later" — there is no wrapper to add it to. Give the decoder a `decode_*_proto(&[u8])`
sibling if it lacks one; the message-level `decode_x(&ResponseMessage)` form exists only for
`StreamDecoder` impls now.

**There are two one-shot helpers, and both retry.** `one_shot_request_with_retry` for a
request-id request, `one_shot_with_retry` for a shared channel. Neither takes a
`ProtocolFeature`: call `check_version(..)?` on the line above, as every version-gated one-shot
already does. A one-shot that does not retry is a bug, not a choice — see below.

**Nothing in the type system enforces this**, and treating the signature as the guarantee is the
mistake this node exists to prevent. `expected` and `decode` are unrelated — `R` is inferred from
the decoder — so `expect_proto(IncomingMessages::UserInfo, decode_family_codes_proto)` compiles
and feeds one message's bytes to another's prost type. The helpers also still accept a bare
`impl Fn(&ResponseMessage)`, so skipping the narrow compiles too.

The gate is `test_expect_proto_sites_match_the_roster` (`src/common/one_shot_pairing_tests.rs`).
It scrapes every `expect_proto` site out of `src/` and checks each pair against a declared
`PAIRS` roster, both directions, plus `SITES_PER_PAIR == 2` — sync and async each spell the pair
once, and a count of 1 means they have drifted. **Adding a one-shot API means adding a `PAIRS`
line.** That is the standing cost of keeping the pairing outside the type system.

Do not reach for `expect_proto` inside `impl StreamDecoder::decode`; see
[proto-only decoding](proto-only-decoding.md) for what that surface owes instead.

## Why

Narrowing used to be opt-in. Each domain hand-wrote a `decode_*_message` wrapper doing
`expect_type(..)?` before its real decoder, and 24 of the 44 one-shot sites had no wrapper at
all — they decoded whatever arrived on the channel. The follow-up that scoped #738 put that
number at four; the real one was six times larger, which is the usual shape of a
completeness claim nobody ran a command for.

The consequence is quiet. A foreign proto handed to the wrong `prost` type usually decodes to
*something* — field numbers overlap across messages — so the caller gets a plausible struct full
of wrong values rather than an error.

`expect_proto` is a combinator returning `impl Fn(&ResponseMessage)` rather than a fifth
parameter on the two one-shot helpers, because both already carry four or five arguments against
a budget of three (see [param budget](../style/param-budget.md)) and neither has a builder in
front of it.

The honest end state is a `trait ProtoPayload { const MESSAGE_ID; fn decode(&[u8]) }` implemented
once per payload, which makes `expect_proto::<T>()` take no literals and retires both the roster
and this node's standing cost. Tracked in
[plans/claude-md-knowledge-graph.md](../../../plans/claude-md-knowledge-graph.md).

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
  the same mutation now fails the roster by name.
- #740 — narrowed `StreamDecoder::decode` to `&ResponseMessage`, which is what let the four
  option-computation sites stop hand-rolling `send_raw` + fold. Their decoder wanted `&mut`, and
  the retrying helper's processor bound is `Fn(&ResponseMessage)`; the `&mut` turned out to be
  vestigial rather than a real constraint.
- #741 — deleted `one_shot_request` and `fold_one_shot_mut`, so every one-shot retries and no
  hand-rolled folds remain. The retry gap it closed was invisible in the signature: the helper
  that carried the version check was the one without retry.
