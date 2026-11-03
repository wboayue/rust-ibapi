---
name: live-probe
description: Verify actual TWS wire behavior against the paper gateway before typing a field, constraining a value, documenting a default, or closing a live check. Covers preflight (clock, gateway, paper guard), probe shapes, hang triage, and where findings go.
argument-hint: "[question to settle, e.g. 'is OrderStatus.filled ever absent?']"
---

# Live probe

Use when the question is "what does TWS actually send/accept", not "does our code handle X".
Docs, C# samples and field names have each been wrong (stale `FundamentalReportType` values,
VOL units, `what_if_type` semantics); the paper gateway is the authority.

State the question as one falsifiable sentence before writing code: `$ARGUMENTS`.

## 0. Cheaper checks first

A probe costs a gateway session and often a market-hours wait. Try these first:

- Grep the captured-wire fixtures in `src/**/*tests.rs` for the field; real TWS values appear
  there as assertions.
- Read the C# reference (`EDecoder.cs` for inbound fields, `EClient.cs` / `EClientUtils` for
  what's sent, `Order.cs` / `Contract.cs` for types). See
  [wire enum typing](../../../docs/rules/wire/enum-typing.md).

Probe when those disagree, are silent, or the question is about what TWS *accepts* (rejection
codes, defaults for absent fields, units). Neither source answers those.

## 1. Preflight

`$S` below is your scratchpad directory.

```bash
TZ=America/New_York date '+%a %H:%M ET'
lsof -nP -iTCP -sTCP:LISTEN | grep -E ':(4002|7497) '   # expect java; ssh = tunnel, remote may be down
```

- **Reset windows**: the gateway accepts TCP but ignores the handshake. Daily 00:15–01:45 ET
  (Sun–Fri), weekend Fri 23:00 – Sat 03:00 ET; other regions in
  [troubleshooting](../../../docs/troubleshooting.md#gateway-maintenance-windows). Wait; don't debug.
- **Market session** decides what you can observe: RTH 09:30–16:00 ET for fills, live
  ticks, PnL. What-if, contract details, option chains and order acceptance work off-hours;
  overnight books are live, so "off-hours" is not "no quote". If the question needs RTH and
  it isn't, add a `- [ ]` line to `plans/live-checks.md` and stop.
- **Paper only.** Port 4002 (Gateway) or 7497 (TWS). In the probe, assert
  `managed_accounts()` ids start with `D` before sending anything that mutates state.
- **Client id**: pick one outside 200–399 (the integration tests' `ibapi_test::ClientId`
  pool) that no other session uses. A 326 "client id already in use" means a clash.
- Log `client.server_version()` — every finding is tagged with it.

## 2. Probe shape

Pick the lightest that answers the question:

| Question | Shape |
|---|---|
| Public API behavior (what a caller sees) | Scratchpad bin crate, `ibapi = { path = "<repo>" }` |
| Raw frames / request ids / encoder-level | Temp `#[ignore]` test inside the domain (encoders are domain-private): `client.send_request` + `next_before(deadline)` loop counting frames |
| Enumerating an `Other` catchall | Diagnostic test printing each `Other` + raw message type, run under several session states (empty, prior open orders, prior subscriptions) |
| Framing / exact bytes | Any of the above with `IBAPI_RAW_CAPTURE_DIR=$S/raw`; walk with `cargo run --example replay_raw_capture -- <file>.bin`. `IBAPI_RECORDING_DIR` for parsed frames |

Probe rules:
- Bound every read with a deadline; on async, check `Instant::now() >= deadline` at the loop
  top (`timeout_at` polls the inner future first and keeps returning queued items).
- Write output to a file and flush after each line, or `eprintln!` + `RUST_LOG=debug`;
  `timeout cmd > file` loses block-buffered stdout on kill.
- Orders: prefer what-if (`.analyze()`). Otherwise non-marketable limit, then cancel. Paper
  fills arrive in chunks; re-read `filled` from the cancel reply before flattening, and
  flatten anything that filled.
- Probe each variant/value separately; record which code each draws (e.g. 321 vs 430).

## 3. Hang triage

In this order — each has been misdiagnosed as a gateway problem:

1. **Zero output, even pre-connect prints** → `sample $(pgrep -f <bin>) 1`. `_dyld_start`
   = macOS holding a freshly built binary (~50s). Give fresh binaries ≥60s before `timeout`.
2. **Output stops at connect, TCP accepts** → re-check the reset window; check the listener
   is `java`, not an `ssh` forwarder.
3. **Outside the window, still silent** → post-reset wedge. Ask the user to restart the
   gateway; don't assume.
4. **Immediate EOF after handshake** → 326 client id clash; pick another id.
5. **`UnexpectedEof` / `WouldBlock` across many connections** → gateway connection limit;
   serialize (`--test-threads=1`).

## 4. Record

Write the finding as you get it, with: date + time ET, session (RTH / off / overnight /
weekend), paper, server version, instrument, request, observed frames and error codes, and
what it settles vs. what the docs/C# claimed. Note what was **not** probed.

- **Durable wire fact** (a field's presence, units, accepted values, error code) →
  `docs/wire-facts/<domain>.md`, in the format its [README](../../../docs/wire-facts/README.md)
  sets. Ships in the PR the fact informs; a probe no PR acts on yet gets its own small docs
  PR. Rewrite an entry a new probe supersedes.
- **Changes how a caller uses an API** → also the item's rustdoc, with a `live: <date>` note.
- **Check waiting on market hours** → `plans/live-checks.md` under `## #N <title>`: `- [ ]`
  while pending, ticked `- [x]` with the line above when run; then move the result to
  `docs/wire-facts/`. Not a GitHub issue
  ([plans and issues](../../../docs/rules/workflow/plans-and-issues.md)).
- **Confirms or contradicts a rule node** → append to its `precedents:`.
- Don't commit probe code; cite the probe in the PR body.

## 5. Clean up

Delete temp `#[ignore]` / diagnostic tests from the tree before the PR (investigation
artifacts, not suite). Cancel open orders and flatten positions the probe created. Scratchpad
crates can stay, but never cite their paths in the repo: they are session-scoped.
