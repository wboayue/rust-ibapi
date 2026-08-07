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

## Reproduction strategy (deterministic, offline)

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
