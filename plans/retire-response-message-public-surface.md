# Retire `ResponseMessage` From the Public Surface

**Status:** ✅ shipped. All three PRs landed: `ResponseMessage` is now
`pub(crate)`, the crate-root re-export is gone, `StartupMessage::Other` is
removed, and `Error::UnexpectedResponse` carries a `String` instead of the
wire envelope. Synthesized notice taxonomy
(`HANDSHAKE_UNKNOWN_FRAME_CODE = -3`, `HANDSHAKE_DECODE_FAILURE_CODE = -4`)
+ `Notice::is_handshake_synthetic()` predicate landed in PR 3 (#581).

**End goal:** `ResponseMessage` becomes `pub(crate)` in 3.0. The crate-root
re-export at `src/lib.rs` is dropped; downstream code can no longer observe
the raw wire envelope. Replace `StartupMessage::Other(ResponseMessage)` —
the one forcing constraint — with typed variants for every message kind that
actually arrives in that branch.

**Parent:** follow-up from
[`plans/hide-internal-types.md`](hide-internal-types.md) (PR 3, #577). The
audit there found exactly one external API surface keeping `ResponseMessage`
public: `StartupMessage::Other(ResponseMessage)` at
`src/connection/common.rs:40`.

## Audit (2026-05-17, against `main`)

`StartupMessage::Other` is hit by `dispatch_unsolicited_message`
(`src/connection/common.rs:239-295`) when a handshake-time message isn't
one of the explicitly-decoded kinds:

| Today | Decoded into typed variant | Falls through to `Other` |
|---|---|---|
| `Error` | (consumed inline, emitted to `notice_sink`) | — |
| `OpenOrder` | `StartupMessage::OpenOrder(OrderData)` | only on decode failure |
| `OrderStatus` | `StartupMessage::OrderStatus(OrderStatus)` | only on decode failure |
| `OpenOrderEnd` | `StartupMessage::OpenOrderEnd` | — |
| `AccountValue` / `PortfolioValue` / `AccountUpdateTime` / `AccountDownloadEnd` | `StartupMessage::AccountUpdate(AccountUpdate)` | only on decode failure |
| **`_` catch-all** | — | **every other `IncomingMessages` variant** |

The existing doc-comment on `Other` (`src/connection/common.rs:37-39`) cites
the expected catch-all kinds: `ExecutionData`, `CommissionReport`,
`CompletedOrder`. Three decoders already exist crate-private:

| Wire kind | Decoder | Return type |
|---|---|---|
| `ExecutionData` | `decode_execution_data` (`src/orders/common/decoders/mod.rs:19`) | `ExecutionData` |
| `CommissionsReport` | `decode_commission_report` (`src/orders/common/decoders/mod.rs:23`) | `CommissionReport` |
| `CompletedOrder` | `decode_completed_order` (`src/orders/common/decoders/mod.rs:27`) | `OrderData` |

End-of-snapshot markers (`CompletedOrdersEnd` ≡ 102, `ExecutionDataEnd` ≡ 55)
are unit kinds (no payload) — see `src/orders/common/stream_decoders.rs:72,89`.

The decode-failure fall-through is defensive padding. Once the catch-all is
gone, decode failures emit a notice via the existing `notice_sink` (same
channel used for `IncomingMessages::Error` at line 251), rather than leaking
the wire envelope to the callback.

### PR 1 capture (2026-05-17)

Ran `examples/async/startup_capture.rs` against a paper gateway with
`client_id = 0` (= configured Master Client ID). Observed handshake order
via `RUST_LOG=debug`:

| # | wire msg_id | kind | dispatch |
|---|---|---|---|
| 1 | 9 | `NextValidId` | consumed in `parse_account_info` |
| 2 | 4 | `Error` (2104/2106/2158-style notice) | `notice_sink.deliver` |
| 3 | 4 | `Error` | `notice_sink.deliver` |
| 4 | 4 | `Error` | `notice_sink.deliver` |
| 5 | 15 | `ManagedAccounts` | consumed; loop exits |

**`Other` arrivals: 0.** No `OpenOrder`/`OrderStatus`/`OpenOrderEnd`/
`AccountUpdate` typed-variant arrivals either — the account had no
outstanding orders, no recent fills, and no active `reqAccountUpdates`.

**Why this is the empty-state baseline, not a counter-example.** The
handshake window in `receive_account_info` closes as soon as `NextValidId`
AND `ManagedAccounts` have both been seen (`src/connection/async.rs:267`).
TWS spec'd order: `NextValidId` → handshake notices → `ManagedAccounts` →
*then* any Master-Client-ID unsolicited replay (open orders +
`CommissionsReport` per `tws-api/docs/content/orders/open_orders.txt:48-50`).
The replay arrives **after** the window has closed and routes via the
normal post-handshake dispatcher, not `dispatch_unsolicited_message`. The
`Other` arm only fires when a non-`NextValidId`/`ManagedAccounts` frame
**races ahead** of the second of those two markers — possible under heavy
Master-Client-ID state, not reproducible in this empty-state run.

**Implication for PR 2's variant set.** Concrete arrivals in `Other`
cannot be enumerated from the live capture alone; we'd need an account
with active Master-Client-ID state to force the race. Two corroborating
data points pin the expected set:

1. **C# Master Client ID docs** explicitly call out unsolicited
   open-order + commission-report replay; commission report =
   `CommissionsAndFeesReport = 59`. Execution + completed-order replays
   aren't explicit in the doc but are plausible adjacent kinds, and the
   existing `StartupMessage::Other` doc-comment (informed but two cycles
   stale) lists them.
2. **PR #513 history** (commit `4de0225`) cites the live-gateway
   diagnostic that produced the original doc-comment listing
   `ExecutionData, CommissionReport, CompletedOrder`.

**Verdict — outcome (a).** Ship PR 2 with the planned variant set:
`Execution(ExecutionData)`, `CommissionReport(CommissionReport)`,
`CompletedOrder(OrderData)`, `ExecutionDataEnd`, `CompletedOrdersEnd`.
The defensive coverage is correct *and* PR 3's catch-all warn+notice is
the right safety net for any kind not in the typed set — which, given
this capture, may include even the doc-comment's claimed kinds if they
consistently arrive after `ManagedAccounts` in practice.

**Capture artifact.** `examples/async/startup_capture.rs` + the
`Cargo.toml` entry are kept for now (small, useful for future
schema-discovery diagnostics on this code path). PR 3 can decide whether
to delete them after the `Other` arm is gone — at that point the example
becomes pure typed-variant logging, which the bigger
`order_update_stream` example already covers.

## Why this matters

`ResponseMessage` is the raw text/proto field cursor every decoder uses
internally. Today's `Other(ResponseMessage)` variant hands that envelope to
downstream code and — per the variant's own doc-comment — asks the caller
to dispatch on `ResponseMessage::message_type()` and "decode as needed,"
re-implementing decoders we already have crate-private. Replacing it with
typed variants does the decode work once, in the right place, and lets
`ResponseMessage` go `pub(crate)`.

## Approach — typed variants, no envelope escape hatch

`StartupMessage` keeps the same dispatch shape, gains typed variants for
the three currently-expected `Other` kinds, and the catch-all goes away.

```rust
#[non_exhaustive]
pub enum StartupMessage {
    OpenOrder(OrderData),
    OrderStatus(OrderStatus),
    OpenOrderEnd,
    AccountUpdate(AccountUpdate),
    // new:
    Execution(ExecutionData),
    CommissionReport(CommissionReport),
    CompletedOrder(OrderData),
    CompletedOrdersEnd,
    ExecutionDataEnd,
    // Other(ResponseMessage) is removed.
}
```

Two policy decisions baked into the shape:

1. **Decode-failure path**: when a typed decoder fails (currently
   fall-through to `Other`), emit a synthesized notice via `notice_sink`
   (`"failed to decode {message_type} during handshake: {err}"`). Same
   channel `Error` frames already use. The user-facing callback never sees
   the raw envelope.
2. **Genuinely-unknown message types**: the `_ =>` arm in
   `dispatch_unsolicited_message` becomes "log at `warn!` + emit a notice."
   The existing log at line 289 says "THIS MESSAGE IS LOST!"; that warning
   is honest — TWS shouldn't be sending unsolicited unknown frames during
   handshake, and if it does, there's nothing typed for the callback to do
   with it. Pair the warning with a notice so downstream observers see it.

`StartupMessage` does **not** carry `#[non_exhaustive]` today
(`src/connection/common.rs:22-24`). PR 2 adds the annotation alongside the
new variants — adding variants is already a breaking change for callers
doing exhaustive matches, so bundling the annotation costs nothing extra
and future-proofs against further unsolicited-at-handshake kinds.

### Why this over alternatives

- **`Other { message_type: IncomingMessages, raw: Vec<u8> }`** — half-measure;
  hides the `ResponseMessage` *type name* but still leaks wire bytes. If
  any downstream caller writes a decoder against `raw`, they're coupled to
  the upstream wire format — exactly what we're trying to avoid. Rejected.
- **Drop `StartupMessage::Other` and silently swallow unknowns** — too aggressive.
  The notice_sink path preserves observability without leaking the envelope.
- **Keep `Other(ResponseMessage)` `#[doc(hidden)]`** — postpones the choice,
  still leaks the type via the public enum variant (rustc forbids `pub`
  variants with `pub(crate)` payloads). Doesn't solve the problem.

## PR breakdown

Three PRs. Each is workspace-green; together they retire the leak.

### PR 1 — capture which `IncomingMessages` actually land in `Other` ✅ shipped (`ca69642`, folded into PR 2)

**Optional gate.** The plan above assumes the three doc-comment kinds
(`ExecutionData`, `CommissionsReport`, `CompletedOrder`) are the complete
set. The doc-comment is informed but two cycles stale. Before adding typed
variants, verify with a live capture:

- Hook a temporary callback into the test harness or a stand-alone
  integration script that records `message.message_type()` for every
  `Other` arrival across a handful of handshakes (paper account with
  outstanding orders, with recent fills, with completed orders).
- Cross-reference the captured set against the C# reference client
  (`/Users/wboayue/projects/tws-api/source/csharpclient/client/EDecoder.cs`)
  to confirm which unsolicited frames TWS spec-emits at handshake time.

**Outcomes:**
- **(a) Only the three documented kinds arrive** → PR 1 is a tiny capture
  note in this plan file; proceed to PR 2 with the variant set above.
- **(b) Additional kinds arrive** → expand the typed-variant set in PR 2's
  scope; if decoders don't exist for the new kinds, ship a precursor PR
  adding them. Same playbook as `typed-status-sweep` PR 5a (branch
  `typed-status-sweep-pr5a-diagnostic`, unmerged; tracker:
  `plans/typed-status-sweep.md:18`).

This PR is a research deliverable, not code. The output is a short table
appended below the audit, listing each observed `IncomingMessages` variant +
frequency + whether a decoder exists.

### PR 2 — add typed variants to `StartupMessage` ✅ shipped (`ca69642`)

Add the variants and `#[non_exhaustive]` together; `Other` stays in place
until PR 3. Each new variant gets:

- An enum arm in `StartupMessage` (`src/connection/common.rs:24`).
- A dispatch arm in `dispatch_unsolicited_message`
  (`src/connection/common.rs:239-295`) that calls the existing decoder.
  Decode failure keeps falling through to `Other(message.clone())` until
  PR 3 replaces that fallback.
- A `message_type()` arm in `impl StartupMessage`
  (`src/connection/common.rs:43-60`) mapping back to the `IncomingMessages`
  variant.
- A test fixture exercising the happy path + decode-failure path. The
  needed builders already exist in `src/testdata/builders/orders.rs`:
  `ExecutionDataResponse` (line 229), `CommissionReportResponse` (144),
  `CompletedOrderResponse` (609), `ExecutionDataEndResponse` (810),
  `CompletedOrdersEndResponse` (848).

**Migration guide entry** (`docs/migration-3.0.md`):

> #### `StartupMessage` gains typed `Execution` / `CommissionReport` / `CompletedOrder` variants
>
> If you matched on `StartupMessage::Other(rm)` and called
> `rm.message_type()` to dispatch on `ExecutionData` / `CommissionsReport` /
> `CompletedOrder`, switch to the new typed variants — the payload is
> pre-decoded:
>
> ```rust,ignore
> // v2.x / current 3.0
> match msg {
>     StartupMessage::Other(rm) if rm.message_type() == IncomingMessages::ExecutionData => {
>         // ... decode rm yourself ...
>     }
>     // ...
> }
>
> // after PR 2
> match msg {
>     StartupMessage::Execution(execution) => { /* typed payload */ }
>     // ...
> }
> ```

### PR 3 — remove `StartupMessage::Other`; narrow `ResponseMessage` to `pub(crate)` ✅ shipped (#581)

The terminal PR. Three coupled outcomes:

**(a) Remove `StartupMessage::Other`.** Delete the variant. Update the
`_ =>` catch-all in `dispatch_unsolicited_message` to log + emit a notice
(replaces the current `cb(StartupMessage::Other(message.clone()));` line).
Update the three decode-failure fall-throughs (OpenOrder, OrderStatus,
AccountUpdate at lines 257, 265, 281) to do the same.

**(b) Drop the crate-root re-export.** Remove `ResponseMessage` from the
`#[doc(inline)] pub use messages::{...}` block in `src/lib.rs:143`.

**(c) Narrow `ResponseMessage` to `pub(crate)`** at `src/messages.rs`. Its
`pub fn` methods (`message_type`, `request_id`, `peek_*`, etc.) become
crate-internal automatically — every internal callsite already uses
`crate::messages::ResponseMessage`, so the narrowing is transparent.

**Sub-decision to resolve in PR 3**: which `Notice` code(s) to use for
synthesized "unknown handshake frame" / "decode failure" notices. The
existing taxonomy has `SYSTEM_MESSAGE_CODES`, `WARNING_CODE_RANGE`
(2100..=2169), `ORDER_REJECTION_CODE_RANGE` (200..=399), and the
client-side `ORDER_CANCELLED_CODE` (202). Two options worth weighing:
  1. Reuse an existing system-message code (e.g. 1300 "TWS connectivity
     change") — semantically loose but no new vocabulary.
  2. Pick a sentinel in the unassigned negative range (e.g. -1 / -2),
     following the existing `-2` shutdown convention
     (`ResponseMessage::is_shutdown`). Honest about origin (client-side
     synthesis) but introduces a new contract.

  Lean: option 2, sentinel codes, documented in `src/messages.rs` next to
  the existing range constants. Decide during PR 3, not now.

**Migration guide entry**:

> #### `StartupMessage::Other` and `ResponseMessage` are crate-private
>
> The `Other(ResponseMessage)` variant is gone. Unsolicited handshake-time
> messages that aren't one of the typed kinds are now reported via
> `Client::notice_stream()` (codes synthesized in the connection layer —
> see release notes for the exact range). If you matched on `Other(_)` to
> log "unexpected handshake frame," subscribe to the notice stream
> instead.
>
> `ibapi::ResponseMessage` is no longer reachable. The type was wire
> plumbing; downstream code never had a reason to reach it.

## Out of scope

- Reshaping `ResponseMessage` itself, splitting it into a per-message-kind
  cursor, or removing the `is_protobuf` branching inside it. PR 3 just
  narrows visibility; the internal API stays the same.
- The other handshake-time leaks audited in `hide-internal-types.md` PR 4
  — `AsyncInternalSubscription` (`pub` inside `pub(crate) mod transport`)
  and `parse_raw_message` (`pub` inside `pub(crate) mod connection`).
  Both are crate-private externally already; tightening their declarations
  is cosmetic.

## Verification per PR

CLAUDE.md "Quick Commands" applies as-is (three clippy configs, three
rustdoc configs, `just test`, integration-crate builds per rule 11). PR 1
is a research deliverable — no quality gates beyond a documented capture.

## Sequencing

PR 1 → PR 2 → PR 3. PR 1 may compress to a one-paragraph "capture confirms
the doc-comment set" if the live run produces no surprises. PR 3 ships only
after every typed variant decoder has shipped (rule 23: modernize callers
first, restrict second — the typed-variants PR is the modernization, PR 3
is the restriction).
