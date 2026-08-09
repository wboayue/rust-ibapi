# Investigation: tick-by-tick AllLast decode desync after data-farm reconnect

**Filed:** 2026-07-20 · **Reported by:** downstream consumer (bumba issue #208) · **Repro version:** 3.1.0; **target:** 3.3.0 (no tick-decode fix in 3.2.0–3.3.0 release notes)

## Symptom

On 2026-07-07 a live `tick_by_tick(&es, 0).all_last()` subscription began emitting corrupt
trades mid-session and **never recovered** (corrupt to session end). The corruption is
**structured, single-field**, not random wire noise:

- **Only `price` is wrong.** At onset it gains a constant additive offset of exactly
  `445,860,400 × 3 = 1,337,581,200`. First corrupt print `445,867,965 = 445,860,400 + 7565`
  (7565 = the true ES price); two further `+445,860,400` steps reach `1,337,588,765`; from
  there **price tracks real ±0.25 tick moves on top of the offset** (step-size histogram is
  almost entirely `0`/`±0.25`). Later a few more large jumps push it toward `3.57e9`.
- **`size` is real and varied** (1–544). **`time` advances normally** through session end
  (only 3 blip `1970-01-01T00:00:0X` stamps at the very onset).
- Downstream, this poisoned every price indicator; not ibapi's concern but confirms severity.

So the decoder reads *most* of each message correctly (real size, real time, real tick
deltas) but adds a large constant to `price` — and does so **without erroring**.

## Correlated trigger (from the consumer's `RUST_LOG=info` log, all UTC)

```
16:26:09.978  WARN [2119] Market data farm is connecting:usfarm.nj   <- ES farm reconnecting
16:26:10.632  INFO [2104] Market data farm connection is OK:usfarm.nj  <- back "OK"
   ...tick-by-tick price corrupt ~16:26:40 (~30s later), permanent...
16:28:17 / 16:30:01 / 16:34:17  usfarm.nj flaps [2108]/[2119]/[2104] for ~8 min
```

A **data-farm reconnect** (`usfarm.nj`, the farm carrying ES) at 16:26:09–10 preceded the
corruption by ~30 s. ibapi logged the farm notices (`ibapi::transport::common`) but **no
decode error** — from its view the reconnect "succeeded."

Key distinction: a farm reconnect is a **2100-band warning on the persistent TCP socket**
(`FARM_*_CODES` in `src/messages.rs:1141-1154`), *not* a socket teardown and *not* the
1100/1101/1102 connectivity-lost path (`src/messages.rs:1111-1126`). So the read loop keeps
consuming the same stream across the transition — no resubscribe, no decode-state reset.

## Hypotheses (ranked)

- **H1 — length-prefix framing desync on the persistent socket (most likely).**
  During the farm transition IB emits a partial/short/reset frame; the 4-byte length or
  msg-id read (`src/connection/common.rs:383-390`, `i32::from_be_bytes([data[0..4]])`) misaligns
  by a fixed byte count, so every subsequent `read_message()` (`src/connection/async.rs:185`,
  routed at `src/transport/async.rs:377`) starts mid-message. A *fixed* boundary shift explains
  the *constant* offset: the `price` fixed64 is read from a consistently-shifted position, its
  low bits still tracking the real field. Never recovers because nothing re-syncs the framing.
  **This is the primary suspect — audit the length read + any place a short read / partial
  frame could be swallowed rather than surfaced.**

- **H2 — prost field/oneof mis-decode.** `TickByTickData` is a `oneof` (`src/proto/protobuf.rs:636-645`,
  tags 3/4/5). If a post-reconnect message decodes as the wrong variant or a preceding
  varint is mis-sized, `price` could shift. Less able to explain a *stable additive constant*
  with correct tick deltas, but must be excluded.

- **H3 — no decode/buffer reset on farm reconnect.** Whatever buffering sits between the socket
  and the parser may retain stale bytes across the farm transition. Confirm whether any
  read-buffer state survives a 2100-band event, and whether it *should* be flushed/re-synced.

- **H4 — stateful/accumulating decode.** The "+445,860,400 accrues 3× then sticks" pattern
  hints at an accumulator or a running value fed into `price`. Check for any additive/delta
  handling in the tick path (unlikely in a stateless prost decode, but the 3× ramp is
  anomalous enough to rule out explicitly).

## Investigation steps

1. **Nail the desync locus (H1).** Read `src/connection/common.rs` framing (383-390 and the
   length decode), `src/connection/async.rs:185 read_message`, and `src/transport/async.rs:377
   read_and_route_message`. Find every path where a short read, zero-length frame, or decode
   failure is **dropped/logged rather than surfaced as a re-sync** — that's where a one-time
   misframe becomes permanent.
2. **Analyze the constant.** `445,860,400` (and the 3× ramp). Decompose against fixed64/varint
   field widths in the tick message to see which byte-shift or field reinterpretation yields
   exactly this additive constant. The number *is* the fingerprint of the mis-read.
3. **Exclude H2/H4** by decoding a single known-good AllLast message, then the same bytes
   shifted by N, and checking which shift reproduces `real_price + 445,860,400`.
4. **Confirm H3** by tracing buffer/state ownership across a simulated 2100-band notice.

## Framing audit (2026-08-08) — step 1 done

Read: `src/connection/common.rs` (`parse_raw_message`), `src/connection/{sync,async}.rs`
(`read_message`), `src/transport/sync.rs:849-868` (`read_header`/`read_message`),
`src/transport/async/io.rs:56-74` (`AsyncIo for AsyncTcpSocket`), `src/transport/routing.rs`,
`src/transport/recorder.rs`, and both dispatcher loops
(`src/transport/sync.rs:270-313`, `src/transport/async.rs:325-391`).

**F1 — `parse_raw_message` panics on a frame shorter than 4 bytes. Confirmed by test.**
`src/connection/common.rs:391` indexes `data[0]..data[3]` with no length check. Both readers
accept a length prefix of 0–3 (`read_exact` into a 0-length buffer succeeds), so the body
reaches `parse_raw_message` and panics: `index out of bounds: the len is 0 but the index is 0`.
Kills the sync dispatcher thread / async dispatcher task — not a graceful `Error`.

**F2 — no `MaxMsgSize` bound on the length prefix. This is the H1 mechanism.**
Sync `read_header` and async `read_message` accept any `u32` and immediately
`vec![0u8; message_length]`. The C# reference guards exactly this:
`if (msgSize > Constants.MaxMsgSize) throw new EClientException(EClientErrors.BAD_LENGTH)`
(`EReader.cs:120`), `MaxMsgSize = 0x00FFFFFF` (16 MiB, `Constants.cs:17`). Two consequences
from four garbage bytes: a zeroed allocation of up to 4 GiB, and — the important one — the
subsequent `read_exact` blocks until that many bytes arrive, **consuming and destroying every
real message in between**, then returns one giant bogus frame. The next `read_header` starts at
another arbitrary boundary. Nothing re-anchors. **That is the "never recovered" signature.**
Cheapest, highest-value fix; take it first.

**F3 — a desynced frame is dropped in total silence.**
Garbage `msg_id > 200` → `real_type = msg_id - 200` → `IncomingMessages::from` → `NotValid`
(`= -1`, `src/messages.rs:33,300`) for anything unrecognised → `determine_routing` falls
through to `ByMessageType(NotValid)` → shared channel nobody subscribed to → dropped. No log,
no error, no counter; nothing anywhere counts unroutable frames. Matches the report exactly:
farm notices logged, no decode error.

**F4 — the corruption itself needs no separate hypothesis; H1 produces it.**
When the garbage `msg_id` happens to land on a *valid* type, the shifted payload goes to prost,
which is lenient: unrecognised field numbers are skipped and a `fixed64` read at a shifted
offset yields a plausible number, so `decode` returns `Ok`. That is a wrong `price` with sane
`size`/`time` and no error. **H2 is subsumed** — it is the same defect, not an alternative.

**F5 — H3 excluded, and the reason it can't self-heal.**
No read buffering survives anything: both readers `read_exact` into a fresh `Vec` per frame
(sync has no `BufReader`; async reads `OwnedReadHalf` directly). No retained parser state to
flush. But that is also *why* nothing re-syncs — the framing is purely positional, with no
delimiter or magic to re-anchor on after a slip.

**F6 — H4 excluded.** The tick path is stateless: `decode_tick_by_tick_*` reads
`data.price.unwrap_or_default()` (`src/market_data/realtime/common/decoders/mod.rs:268,285`).
No accumulator anywhere. The 3× ramp remains unexplained — still a note, still not a blocker.

**F7 — the repro strategy below is wrong about the recorder. Read this before capturing.**
`MessageRecorder` is **not** a raw wire tap. `record_response`
(`src/transport/recorder.rs:70-95`) is called on the **already-parsed** `ResponseMessage` and
re-synthesises a frame via `encode_protobuf_message(message.message_type() as i32, payload)`.
Two losses that matter here:
- **the 4-byte length prefix is never recorded** — and that prefix is precisely what a framing
  desync corrupts;
- the msg-id round-trip is lossy — an unrecognised type collapses to `NotValid = -1` and is
  re-encoded as `199`, destroying the garbage id that would identify the shift.

So a `IBAPI_RECORDING_DIR` capture **cannot** reproduce a framing desync. Capturing one needs a
byte-level tap: `tcpdump` on the gateway port, or a raw-frame tee inside
`AsyncIo::read_message` / `Io::read_message` before unframing. Treat "add a raw-frame recording
mode" as a precursor step, not part of the fix.

### Recommended sequencing

1. ~~**Fix F1 + F2**~~ — **done.** `validate_frame_length` in `src/transport/common.rs` bounds
   the prefix at `MAX_FRAME_LENGTH` (`0x00FFFFFF`, matching C# `Constants.MaxMsgSize`) and
   rejects bodies below `MIN_FRAME_LENGTH`; both frame readers call it, and
   `parse_raw_message` guards the header slice instead of indexing it. Out-of-range prefixes
   raise the new `Error::InvalidFrame`, which `is_connection_lost` reports as true so both
   dispatchers take their reconnect branch. Regression tests at all three seams.
2. **Then F3** — surface unroutable frames (log + counter, or a synthesized notice) so the next
   desync is observable instead of silent. **Next up.**
3. **Then the raw-frame tap (F7)** if a live capture is still wanted to confirm the trigger.

**Still open after step 1:** F2 is the most plausible root cause but is *not confirmed* as the
one that fired on 2026-07-07 — no capture exists, and F7 explains why one could not have been
taken. If the corruption recurs on a build carrying this fix, the `Error::InvalidFrame` +
reconnect is the expected new symptom; corruption *without* an `InvalidFrame` would falsify F2
and put H2-by-another-route back in play.

## Reproduction strategy (deterministic, offline)

> **Superseded in part by F7** — the recorder-based capture below does not preserve length
> prefixes. Steps 2 and 3 remain valid for decode-level replay; step 1 needs a raw tap.

The wire recorder is the backbone: `src/transport/recorder.rs` (`MessageRecorder`, enabled by
`IBAPI_RECORDING_DIR`, writes `NNNN-response.msg` **raw wire frames**).

1. **Capture live.** Run the consumer (or a minimal `tick_by_tick().all_last()` example) on the
   Chicago paper box with `IBAPI_RECORDING_DIR` set, spanning a data-farm reconnect. The farm
   flaps regularly (07-07 saw several in 8 min), so a multi-hour capture should catch one.
   Correlate the corrupt-frame index with the `[2119]/[2104]` notice frames.
2. **Replay offline.** Feed the recorded raw frames through the decode path in a unit/integration
   test (extend `src/proto/decoders_tests.rs` / `src/transport/*_tests.rs`) — no network, fully
   deterministic. This becomes the **regression test**: the recorded reconnect-window bytes must
   decode to sane prices (or surface an explicit re-sync/error), never `real + 445,860,400`.
3. If live capture is slow, **synthesize** the failure from step-2 analysis: take a good AllLast
   frame, prepend the partial/reset frame the analysis implicates, assert the decoder either
   re-syncs or errors instead of silently offsetting.

## Fix direction (pending root cause)

- **If H1:** make the framing self-synchronizing or fail-loud — a length/msg-id that can't be a
  valid frame must return a recoverable `Error` (see `src/errors.rs:188` `is_stream_recoverable`
  semantics) that tears down and resubscribes, **not** a silently mis-parsed message. Permanent
  silent corruption is the real defect; erroring is acceptable, silent garbage is not.
- **Defensive, independent of root cause:** on a farm-`Broken`/`Connecting` notice for a farm
  backing an active subscription, optionally **invalidate downstream decode state / signal
  resubscribe** (mirror the 1101 "data lost → resubscribe" contract at `src/messages.rs:1111`).
  This turns a silent desync into an observable, recoverable event.
- **Sanity guard (belt-and-suspenders):** a tick-by-tick price wildly outside a plausible band
  is almost certainly a decode fault; consider surfacing it as an error rather than a `Data`
  item. (Decoder-level; the consumer also guards, but the crate shouldn't emit `4.5e8` as a
  valid ES trade.)

## Validation

- Recorded reconnect-window bytes replay to sane prices or an explicit re-sync/error (new
  regression test) — never a constant-offset price.
- `just cover` on touched modules; existing tick-by-tick + transport tests stay green.
- Re-run the live capture post-fix across a farm reconnect: no corruption, or a clean
  resubscribe.

## Out of scope / related

- Consumer-side defenses (sane-band trade guard; consuming the connectivity band to pause/flatten)
  live in bumba #208 / #212 — complementary, not a substitute for the crate fix.
- General reconnect/resubscribe policy beyond this decode path.
- The 3× ramp vs single-shift discrepancy may reveal a second, smaller bug — note but don't
  block the primary fix on it.
