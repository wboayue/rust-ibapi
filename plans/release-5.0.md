# 5.0 release backlog

Breaking changes held back from 4.x minors. Land each one in the 5.0 cycle, and
give it a section in `docs/migration-5.0.md`.

## Done

- utoipa 5 → 6 (#884, supersedes dependabot #870): feature `utoipa-6` over
  internal `utoipa`; dep renamed `utoipa6`, aliased at the crate root.
  Adding utoipa 7: `utoipa-7 = ["utoipa", "dep:utoipa7"]` + a newest-wins
  `cfg` on the `extern crate` aliases in `lib.rs`. Close #870 by hand.
- migration-4.0.md §16–§28 + three behavioral bullets moved to
  migration-5.0.md §1–§13; CHANGELOG pointers updated.
- Hide prost from `Error` (#885): `ProtobufDecode(errors::ProtobufDecodeError)`,
  no public `From<prost::DecodeError>`; decoders use
  `proto::decoders::DecodeProto::decode_proto`. `ParseTime` keeps
  `time::error::Parse` (time is a vocabulary dep); its clone is now lossless.
  Plan in #888's history. migration-5.0.md §15.
