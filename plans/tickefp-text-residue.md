# TickEFP Text-Protocol Residue

`TickEFP` is the only response type still on the text wire, and the C# reference confirms this is **by protocol design**, not a temporary gap. The legacy text-protocol cleanup arc (archive issue [#645](https://github.com/wboayue/rust-ibapi/issues/645)) deliberately stopped here with the floor at `PROTOBUF_REST_MESSAGES_3` (213).

This file tracks the helpers that survive solely to support `decode_tick_efp`. If we decide to drop the API — or IBKR ships a server-side proto encoder for TickEFP — these all become deletable in one pass.

## C# protocol evidence

`/Users/wboayue/projects/tws-api/source/csharpclient/client/EDecoder.cs` does framing-level dispatch at lines 70-77 (identical to our `parse_raw_message`): if `incomingMessage > Constants.PROTOBUF_MSG_ID (200)`, strip the offset and route to the proto switch; otherwise route to the text switch.

- **Proto switch** (lines 79-328): dedicated `*EventProtoBuf(len)` handlers for `TickPrice`, `TickSize`, `TickString`, `TickGeneric`, `TickOptionComputation`, `TickSnapshotEnd`, `MarketDataType`, `TickReqParams`. **No `TickEFPEventProtoBuf` exists** — if TWS sent `msg_id = 247` (200 + 47), C# would hit `default:` at line 325 and emit `EClientErrors.UNKNOWN_ID`.
- **Text switch** (lines 332+): `case IncomingMessage.TickEFP: TickEFPEvent()` is the only path.

Corroborating absences in the C# reference:
- No `TickEFP.proto` in `source/proto/` (every other tick type has one).
- No `MIN_SERVER_VER_*TICK_EFP*` constant in `MinServerVer.cs`.

## Empirical corroboration

Live probe on 2026-05-27 (IB paper Gateway, port 4002, server v220+, floor 213) subscribed to AAPL, SPY, IBKR, SAP@IBIS, SIE@IBIS for ~5s each. 251 frames total, 100% proto-framed, **0 msg_id=47**. The Eurex SSF stocks (SAP, SIE) — most likely TickEFP triggers per IB docs — returned other tick types fine, so the probe wasn't permission-blocked.

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

1. **The carrying cost is no longer worth supporting EFP at all.** Given the C# evidence above, this is the only path under our control. EFP (Exchange For Physical) ticks are a niche surface; US single-stock-futures stopped trading in 2020 and the 2026-05-27 probe found zero EFP frames even on Eurex SSF stocks. Removing `TickTypes::EFP` from the public API + `decode_tick_efp` would free every helper in the inventory above.
2. **`Client::tick_by_tick_*` drops EFP support** for another reason (e.g. API simplification, deprecation of the parent surface).
3. **IBKR adds a server-side proto encoder for TickEFP.** Out of our control — would require a new `TickEFP.proto`, a `MIN_SERVER_VER_*TICK_EFP*` constant in `MinServerVer.cs`, and a `TickEFPEventProtoBuf` handler in `EDecoder.cs`. None of those exist today. Watch for these in future TWS API releases; if they appear, the cleanup is mechanical (add a proto decoder, flip `decode_tick_efp` to `require_proto()`, migrate fixtures, delete the helpers).

## Why this lives in its own file

Extracted from the now-deleted `plans/legacy-text-protocol-cleanup.md` after the arc concluded. The original file documented the per-family ratchet workflow (PRs #527 → #530 → #632, plus per-decoder cleanups #529 / #531 / #532 / #534 / #543 / etc.); that workflow is no longer active. What remains is this single TickEFP-shaped residue.
