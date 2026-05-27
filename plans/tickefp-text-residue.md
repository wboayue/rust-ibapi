# TickEFP Text-Protocol Residue

`TickEFP` is the only response type still on the text wire — TWS has no protobuf encoder for it. The legacy text-protocol cleanup arc (archive issue [#645](https://github.com/wboayue/rust-ibapi/issues/645)) deliberately stopped here with the floor at `PROTOBUF_REST_MESSAGES_3` (213).

This file tracks the helpers that survive solely to support `decode_tick_efp`. If a future TWS release adds protobuf for TickEFP — or we decide to drop the API — these all become deletable in one pass.

## Live helpers blocked by TickEFP

- `ResponseMessage::from(fields: &str)` constructor — `src/messages.rs`
- `ResponseMessage::skip()` — `src/messages.rs`
- `ResponseMessage::next_int` / `next_string` / `next_double` — `src/messages.rs`
- `parse_raw_message` binary-text payload branch — `src/connection/common.rs`
- `decode_tick_efp` itself — `src/market_data/realtime/common/decoders/mod.rs`

`text_request_id_field(IncomingMessages::TickEFP) = Some(2)` in `src/messages.rs` is also load-bearing — it's the only entry that keeps the text-frame branch of `ResponseMessage::request_id()` non-trivial. The companion `routes_by_request_id` derives from it.

## Production callers of `ResponseMessage::from(&str)`

- `src/connection/common.rs` — text branch of `parse_raw_message` (only reached for TickEFP frames at floor 213).
- `src/stubs.rs` — test-only fixture path (`with_responses(Vec<String>)`).

## End-state trigger

Drop these helpers when **any** of:

1. TWS ships a protobuf encoder for TickEFP. Add a proto decoder, flip `decode_tick_efp` to `require_proto()`, migrate fixtures to `proto_response(IncomingMessages::TickEFP, ...)`, then delete the helpers above in one sweep.
2. The `Client::tick_by_tick_*` API stops exposing EFP ticks.
3. The carrying cost is no longer worth supporting EFP at all.

## Why this lives in its own file

Extracted from the now-deleted `plans/legacy-text-protocol-cleanup.md` after the arc concluded. The original file documented the per-family ratchet workflow (PRs #527 → #530 → #632, plus per-decoder cleanups #529 / #531 / #532 / #534 / #543 / etc.); that workflow is no longer active. What remains is this single TickEFP-shaped residue.
