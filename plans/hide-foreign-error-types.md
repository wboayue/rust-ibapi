# Hide prost's error type from `Error` (#885, 5.0)

One PR. Breaking; lands in the 5.0 cycle.

## Scope: prost only

- **prost: hide.** 0.x, every minor is breaking, and `ProtobufDecode` is its
  only public exposure (`proto` is `pub(crate)`). Hiding it decouples prost
  upgrades from `ibapi` majors.
- **time: keep `ParseTime(#[from] time::error::Parse)`.** `time` is a public
  vocabulary dependency (`OffsetDateTime`, `Date` throughout the API); a
  time 0.4 forces an `ibapi` major regardless, so a wrapper buys no
  insulation. Its one real defect, the lossy `Clone`, is fixed directly:
  `time::error::Parse` is `Clone` (time 0.3.41+), so the `Error::Simple`
  collapse was never needed.

## Corrections to the issue

- **A crate-private `From` impl does not exist.** Trait impls are public when
  both types are public; `impl From<prost::DecodeError> for Error` (or for the
  wrapper) would still be API. `?` conversion goes through a crate-private
  helper instead (below).
- Lossy `ParseTime` clone: see above; no wrapper needed.

## Design

In `src/errors.rs`, one opaque newtype, private field:

```rust
/// Failed to decode a protobuf message from TWS.
#[derive(Debug, Clone)]
pub struct ProtobufDecodeError(prost::DecodeError);
```

- Name follows std's verb-object-`Error` order (`ParseIntError`,
  `FromUtf8Error`) and matches the variant. Plain `DecodeError` collides with
  `prost::DecodeError` and is vague in a crate that also decodes text frames.
- Tuple struct with a private field is already opaque; no `#[non_exhaustive]`.
- `Display` delegates to the inner value; `source()` delegates to the inner
  `source()` (transparent wrapper).
- Traits: `Debug`, `Clone`, `Display`, `Error`, plus auto `Send + Sync`.
  No `Copy` / `PartialEq` / `From`: each re-commits to prost's traits.
  `Debug` output names the inner type; rustdoc says Debug format is not a
  contract.
- Auto traits leak through the concrete field: if a future prost made its
  error `!Sync`, ours would follow and `ibapi::Error` would stop being
  `Send + Sync` (C-GOOD-ERR). Guard with `crate::tests::assert_send_and_sync`
  for the newtype **and** `Error` (no such assertion exists today). Rejected:
  `Arc<dyn Error + Send + Sync>`, an alloc + vtable for a risk the test
  already catches.
- No public constructor: downstream tests can no longer fabricate the
  variant. Accepted (rare); noted in the migration guide.
- Reachable as `ibapi::errors::ProtobufDecodeError` (`errors` is already
  `pub mod`); no root re-export.

Variant (name unchanged, `#[from]` dropped):

```rust
#[error("protobuf decode error: {0}")]
ProtobufDecode(ProtobufDecodeError),
```

Display text unchanged. **No** `#[source]`: an error either renders its
source in `Display` or returns it from `source()`, never both, or chain
printers (`anyhow` `{:#}`, `eyre`) print the decode message twice. Today's
`#[from]` + `{0}` does both. Keep `{0}` (most callers print only `{}`);
`Error::ProtobufDecode(_).source()` becomes `None` (migration note).

## Crate-private conversion

~46 `crate::proto::X::decode(..)?` sites (41 `bytes`, 5
`message.require_proto()?`) in 13 files. A `pub(crate)` blanket trait in
`src/proto/decoders.rs`, beside the other decoder helpers:

```rust
pub(crate) trait DecodeProto: prost::Message + Default {
    fn decode_proto(buf: &[u8]) -> Result<Self, Error> {
        Self::decode(buf).map_err(Error::protobuf_decode)
    }
}
impl<M: prost::Message + Default> DecodeProto for M {}
```

- `pub(crate) fn Error::protobuf_decode(e: prost::DecodeError) -> Error` in
  `errors.rs` keeps the newtype's field private to its module.
- Sites become `X::decode_proto(bytes)?`. **Don't sed on `::decode(`**: the
  crate's own `StreamDecoder::decode` / `TickDecoder::decode` share the name
  (`T::decode(&self.context, &message)` in `subscriptions/{sync,async}.rs`,
  `OptionComputation::decode(&self.decoder_context(), ..)` in
  `contracts/{sync,async}.rs`, `T::decode(&message)` in
  `historical/common/tick.rs`). Anchor on `crate::proto::[A-Za-z]+::decode(`
  or let the compiler drive: after `#[from]` goes, every prost `?` site is an
  error and nothing else is. Fix imports per compiler (`use prost::Message`
  → `use crate::proto::decoders::DecodeProto` where `Message` is unused).
- Untouched: `.ok()` sites in `messages.rs` / `transport/routing.rs`, and
  test-only `T::decode(..).unwrap()` in `common/test_utils.rs`.
- Removing `#[from]` makes every missed site a compile error. Residual
  bypass: `X::decode(b).map_err(|e| Error::Simple(..))` compiles; review
  catches it once the docs show `decode_proto` (below).

## Clone

- `ProtobufDecode(e) => ProtobufDecode(e.clone())`.
- `ParseTime(e) => ParseTime(*e)`. `.clone()` trips `clippy::clone_on_copy`
  (`Parse` is `Copy`). If time ever drops `Copy`, this fails to compile
  rather than silently changing behavior.
- Rewrite the comment above `impl Clone for Error`: only `std::io::Error`
  needs the manual impl.

## Tests (`src/errors_tests.rs`)

- `from_protobuf_decode_error` → assert `TickPrice::decode_proto(&[0xff, 0xff])`
  yields `ProtobufDecode` with `.contains("protobuf decode error")`.
- `protobuf_decode_error()` fixture returns the wrapped `Error` via
  `Error::protobuf_decode`.
- `clone_collapses_parse_time_to_simple` → `clone_preserves_parse_time`:
  cloned value matches `Error::ParseTime(_)`, same Display.
- Add `ParseTime` to the clone round-trip list (~line 259).
- `Error::ProtobufDecode(_).source()` is `None`.
- `assert_send_and_sync::<Error>()` and `::<ProtobufDecodeError>()`, plus
  `'static` (`Box<dyn Error + Send + Sync>` conversion compiles).
- Unchanged and still passing: `error_display` (line 37), `from_parse_time_error`,
  `connection/common_tests.rs:720,731`, `historical/common/decoders/tests.rs:268`.

## Docs

- `docs/migration-5.0.md` §15 "`ProtobufDecode` carries a crate-owned error
  type": before/after snippet; `From<prost::DecodeError> for Error` removed
  (map with `Error::Simple(e.to_string())` or your own error type); payload
  can't be constructed outside the crate; `source()` is now `None` (message
  still in `Display`).
- CHANGELOG:
  - `### Changed`: `ProtobufDecode` payload, pointing at §15 (#885).
  - `### Fixed`: cloning `Error::ParseTime` keeps the variant instead of
    becoming `Error::Simple` (#885).
- Comment on #885 explaining why `time` stays (vocabulary dep, no
  insulation gained).
- `plans/release-5.0.md`: move to Done on merge.
- `docs/extending-api.md:177`: the decoder template shows
  `crate::proto::MyResponse::decode(bytes)?`, which stops compiling. Switch
  it to `decode_proto` and say why (prost's error type stays crate-private).
- Rustdoc on the variant and newtype; `RUSTDOCFLAGS=-D warnings cargo doc`.

## Out of scope

- `time-tz` exposure via `Client::time_zone()` (separate issue).
- Folding `X::decode_proto(message.require_proto()?)` into a
  `ResponseMessage` method: tempting, but a separate cleanup.

## Checks

`cargo fmt`, clippy with CI flags (both feature sets), `cargo test` sync +
async, `cargo doc` with `-D warnings`.
