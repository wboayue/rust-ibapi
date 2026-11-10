---
name: add-tws-feature
description: Port a TWS API request, response, or field from the IBKR C# reference client into ibapi — read EClient/EDecoder, classify the request shape, register the message, encode/decode, type the fields, test, document. Use when adding a client method or a new proto field.
argument-hint: "[C# method or field, e.g. reqUserInfo or Order.conditionsIncludeOvernight]"
---

# Add a TWS API feature

Target: `$ARGUMENTS`. The worked example with code is
[Extending the API](../../../docs/extending-api.md#adding-new-api-functionality) (steps 1–9);
this skill is the order of work and the checks around it. Ship with `/ship`.

**C# reference**: `tws-api/source/csharpclient/client/` in the IBKR TWS API distribution (the
maintainer's checkout: `/Users/wboayue/projects/tws-api/`). Protos: `tws-api/source/proto/`.
It is authoritative for encoding and version gates. It is not authoritative for semantics
(units, defaults, which values TWS accepts): for those, `/live-probe`.

## 1. Read the reference

Write these down before touching code:

| Find | Where in C# | Gives you |
|---|---|---|
| Request method (`reqFoo`) and its version guard | `EClient.cs`: `serverVersion < MinServerVer.X` | Gate value (`MinServerVer.cs`) |
| Proto request builder | `EClientUtils.cs`: `createFooRequestProto` | Which fields are set, and when (e.g. `transmit` only if true) |
| Cancel method (`cancelFoo`) | `EClient.cs` | Whether it's cancellable; IB sometimes reuses a related gate for cancels |
| Response handler(s) | `EDecoder.cs`: `FooEventProtoBuf` | Inbound message ids (`IncomingMessage.cs`), field order, end marker |
| Absent-field handling | `EDecoder.cs` / `EDecoderUtils.cs` | Early return = frame dropped; `if (proto.HasX)` = delivered with a default |
| Outgoing id | `OutgoingMessages.cs` | `OutgoingMessages` variant |
| Types | `Contract.cs`, `Order.cs`, ... | Plain `string` with no enum = likely free-form |

For algo parameters, the Testbed sample (`samples/CSharp/Testbed/AvailableAlgoParams.cs`)
is the wire spec; the docs page has conflated algos. For `List<TagValue>` options, protobuf
encodes a `map<string,string>`.

## 2. Gate and proto

- **Gate ≤ 213** (`server_versions::PROTOBUF_REST_MESSAGES_3`, the handshake floor): no check.
- **Gate > 213 on a request**: add the constant to `src/server_versions.rs`, a
  `ProtocolFeature` on `Features` (`src/protocol.rs`), and
  `check_version(self.server_version(), Features::FOO)?` before sending.
- **Gate > 213 on a field**: check only when the caller sets it, beside its siblings. Order
  fields: a local `ProtocolFeature` in `orders/common/verify.rs`
  (`if order.conditions_include_overnight { check_version(..)? }`).
- Either way, tests derive the boundary from the constant
  ([derive from constants](../../../docs/rules/testing/derive-from-constants.md)).
- **Gate > advertised max** (`max_version` in `src/connection/common.rs`): raise it. TWS then
  starts sending every field gated between the old and new max, so check each against our
  decoders in the same PR (#1045 raised it to 226).
- **Proto missing from `src/proto/protobuf.rs`**: `just proto` regenerates it from IBKR's
  latest protos, so it can carry unrelated upstream changes. Read the regenerated diff: a
  changed or removed field elsewhere is its own change. Commit the regeneration on its own so
  the feature diff stays reviewable.

## 3. Classify the shape

| Shape | Signs in C# | Our pattern |
|---|---|---|
| One-shot keyed by request id | Request carries `reqId`, one answer | `one_shot_by_request_id` + `expect_proto` |
| One-shot, shared channel | No `reqId` in the answer | `one_shot_shared`; `one_shot_shared_matching` if frames can belong to another caller |
| Cancellable one-shot | One answer **and** a `cancelFoo` | `one_shot_cancelling::<K>` |
| Stream | Repeated frames, maybe an `End`, a `cancelFoo` | `StreamDecoder` + `Subscription<T>` |
| New field on an existing message | `if (serverVersion >= X)` in a builder/decoder | Extend the encoder/decoder and the domain type only |

Rules for each: [one-shot narrowing](../../../docs/rules/wire/one-shot-narrowing.md),
[proto-only decoding](../../../docs/rules/wire/proto-only-decoding.md). A stream's End marker
comes after the initial dump only, not after every push
([wire facts](../../../docs/wire-facts/protocol.md#end-markers-come-after-the-initial-dump-only)).

## 4. Implement

Order: types → message ids → registration → encoder → decoder → sync → async.

- **Layout**: the domain owns it: types in `<domain>/mod.rs` (ungated), codecs in
  `<domain>/common/{encoders,decoders}.rs`, `impl Client` in `<domain>/{sync,async}.rs`
  ([domain module layout](../../../docs/rules/style/domain-module-layout.md)). Shared logic
  lives once in `common/`, never copied across sync and async.
- **Registration**: miss any of these and the stubbed tests stay green while a live gateway
  hangs or drops frames:
  - `IncomingMessages` / `OutgoingMessages` variants in `src/messages.rs`
  - `text_request_id_field` in `src/messages.rs` for every inbound type that carries a request
    id. It is the routing allow-list; without it the frame never reaches the subscription
    ([proto-aware accessors](../../../docs/rules/wire/proto-aware-accessors.md))
  - one-shot: the payload in `impl_proto_payload!` (`src/proto/payload.rs`)
  - stream: every handled type in `RESPONSE_MESSAGE_IDS`, plus the decoder in the roster
    checked by `decoder_roster_is_complete`
  - request ids come from `mint_request_id()`, never a caller
    ([id partition](../../../docs/rules/wire/id-partition.md))
- **Encoder helpers**: reuse `ToField` (`src/messages/to_field.rs`) and `some_str` & co.
  (`src/proto/encoders.rs`); add missing ones there, not locally. Mirror `EClientUtils`'s
  "set only when" conditions exactly.
- **Field typing** ([wire enum typing](../../../docs/rules/wire/enum-typing.md)):
  - Required (`Error::Parse` when absent) only if C# drops the frame, or a default would
    fabricate a plausible wrong value. Otherwise tolerate: `Option`, or an open enum's
    `Unknown`/`Other("")`. A decode error ends a long-lived stream.
  - Enum only after the fixtures or C# show a fixed vocabulary; on a streaming path, open
    (`fallback Unknown`).
- **Public API shape**: three parameters or fewer, else a builder
  ([param budget](../../../docs/rules/style/param-budget.md)); one named setter per enum
  variant ([builder enum coverage](../../../docs/rules/style/builder-enum-coverage.md)).
  Parameters every caller must pass go in the entry point, not a setter.

## 5. Test

- Fixtures in `src/testdata/builders/<domain>.rs`, field-minimal, free-function entry points
  ([fixture builders](../../../docs/rules/testing/fixture-builders.md)); tests in sibling
  `*_tests.rs` files ([sibling test files](../../../docs/rules/testing/sibling-test-files.md)).
- Both sides: a sync and an async test through the real `impl Client` method, asserting the
  encoded request and the decoded result
  ([exercise production code](../../../docs/rules/testing/exercise-production-code.md)).
- Version gate: one test below the constant (error) and one at it.
- An absent-field case per field: `Error::Parse` for required ones, `None` / unset for the rest.

## 6. Document

- Rustdoc `# Examples` on each new `pub fn` and builder entry point, sync and async, each async
  example mirroring its sync sibling
  ([public API examples](../../../docs/rules/docs/public-api-examples.md)). Check what a user
  copying it would see: an example that subscribes and then drops shows nothing.
- Gate in the rustdoc ("Requires server version N"), and units where they apply.
- Runnable example in `examples/sync/` and `examples/async/`, each with an `[[example]]` entry
  in `Cargo.toml` (`required-features`). Run it against the paper gateway once: stubbed tests
  can't catch a missing routing entry.
- `CHANGELOG.md` under `## [Unreleased]`
  ([changelog entry](../../../docs/rules/docs/changelog-entry.md)).

## 7. Verify live

An integration test in `integration/{sync,async}/tests/<domain>.rs` when the API has
observable behavior on a paper account ([integration tests](../../../docs/integration-tests.md)).
For anything C# didn't settle (units, accepted values, absent-field defaults), `/live-probe`,
and record the result in `docs/wire-facts/`.

Then `/ship`: wire changes also need the integration crate builds.
