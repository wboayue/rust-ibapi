# sync `Client::drop` — fast shutdown via `crossbeam::select!`

Tracking: issue #523. Replace 1s `recv_timeout` poll in sync cleanup thread
with `crossbeam::select!` over the existing signal channel and a new
shutdown-notify channel. Cleanup thread exits within microseconds of
`request_shutdown` instead of up to 1s.

## Problem (recap)

`src/transport/sync/mod.rs:484-511` — `start_cleanup_thread` blocks on
`signal_recv.recv_timeout(timeout)` (1s, set at `client/sync.rs:139`) and
only checks `is_shutting_down()` after the recv returns. `Client::drop`
→ `ensure_shutdown` → `join` waits up to ~1s for the in-flight recv to
time out. Async side already uses notify; sync should match.

Per-test cost ~1.1s observed in PR #522's algo integration suite.

## Approach

Option 2 from the issue: dedicated shutdown-notify channel, `select!` in
the cleanup thread. Rejected option 1 (drop timeout to 100ms) — still
polls, still has a worst-case lag, and option 2 is the same shape as the
async side.

Keep it minimal: one new channel pair on `TcpMessageBus`, no new types,
no helper struct. Bounded(1) so duplicate `request_shutdown` calls
(reset paths, double-drop safety) collapse via `try_send`.

## Changes

### 1. `src/transport/sync/mod.rs` — `TcpMessageBus` fields (~line 142)

Add:

```rust
shutdown_send: Sender<()>,
shutdown_recv: Receiver<()>,
```

In `TcpMessageBus::new` (~line 159) initialize with `channel::bounded(1)`.

### 2. `request_shutdown` (~line 182) — wake cleanup thread

After `self.shutdown_requested.store(true, Ordering::Relaxed);` add:

```rust
let _ = self.shutdown_send.try_send(());
```

`try_send` (not `send`): bounded(1) returns `Err(Full)` on the second
call when shutdown is already pending — that's the desired idempotent
behavior, drop the error.

### 3. `start_cleanup_thread` (~line 484) — `select!` rewrite

```rust
fn start_cleanup_thread(self: &Arc<Self>) -> JoinHandle<()> {
    let message_bus = Arc::clone(self);
    thread::spawn(move || {
        let signal_recv = message_bus.signals_recv.clone();
        let shutdown_recv = message_bus.shutdown_recv.clone();
        loop {
            crossbeam::select! {
                recv(signal_recv) -> signal => match signal {
                    Ok(Signal::Request(id))       => message_bus.clean_request(id),
                    Ok(Signal::Order(id))         => message_bus.clean_order(id),
                    Ok(Signal::OrderUpdateStream) => message_bus.clear_order_update_stream(),
                    Err(_) => { debug!("cleanup signal channel closed"); return; }
                },
                recv(shutdown_recv) -> _ => {
                    debug!("cleanup thread exiting");
                    return;
                }
            }
        }
    })
}
```

- Drops the `timeout: Duration` parameter.
- Drops the post-recv `is_shutting_down()` check — `select!` fires the
  shutdown arm directly, no flag round-trip needed.
- Bias is unspecified; if both arms have messages crossbeam picks one
  fairly. Pending work signals get drained by the next loop iteration
  if shutdown was picked first — acceptable since we're tearing down.

### 4. `process_messages` (~line 513) — drop the `timeout` param

```rust
pub(crate) fn process_messages(self: &Arc<Self>, _server_version: i32) -> Result<(), Error> {
    let handle = self.start_dispatcher_thread();
    self.add_join_handle(handle);
    let handle = self.start_cleanup_thread();
    self.add_join_handle(handle);
    Ok(())
}
```

The dispatcher thread's `TWS_READ_TIMEOUT` (line 28) is a separate
socket-read concern — leave it alone, this issue is only about cleanup.

### 5. `src/client/sync.rs:139` — drop the `Duration::from_secs(1)` arg

```rust
message_bus.process_messages(connection_metadata.server_version)?;
```

### 6. Existing tests calling `process_messages(sv, Duration::from_secs(0))`

`src/transport/sync/tests.rs:392, :565` — drop the duration argument.
The `0` was a vestigial fast-poll for tests; with `select!` the cleanup
thread idles cheaply with no timeout, no test-mode tweak needed.

## New test

`src/transport/sync/tests.rs` — add a unit test that drives the real
cleanup-thread shutdown and asserts wall-clock < 100ms:

```rust
#[test]
fn cleanup_thread_exits_promptly_on_shutdown() {
    let (_stream, bus) = make_bus();          // existing helper
    bus.process_messages(0).unwrap();
    let start = std::time::Instant::now();
    bus.ensure_shutdown();                    // request_shutdown + join
    assert!(start.elapsed() < std::time::Duration::from_millis(100),
            "cleanup-thread join took {:?}", start.elapsed());
}
```

`make_bus()` (line 609) already provides a `MemoryStream`-backed bus —
lightest fixture that exercises the real `start_cleanup_thread`. No new
fixture infra needed.

Per project rule 13, the test goes in the existing sibling `tests.rs`
(not a new file, this codebase already has the convention there).

## Validation

- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets --features sync -- -D warnings`
- `cargo clippy --all-features`
- `cargo test --features sync` (covers transport tests)
- `cargo build -p ibapi-integration-sync --tests` (rule 16: transport
  plumbing change, cheap to compile-check the integration crate)
- Optional sanity: re-run a slice of PR #522's algo integration suite
  and confirm per-test wall-clock drops from ~1.5s toward ~0.4s
  (matching async).

## Risks / edge cases

- **`reset()` on reconnect** (line 198) does NOT call `request_shutdown`,
  so the shutdown channel stays empty and the cleanup thread keeps
  serving signals across reconnects. No change.
- **Double `request_shutdown`**: bounded(1) + `try_send` makes the second
  call a no-op when shutdown is already pending. Idempotent.
- **Signals pending at shutdown**: if `select!` fires the shutdown arm
  first while signal traffic is queued, the queued cleanup work is
  abandoned. The bus is being torn down; the inserted entries die with
  the `SenderHash`. Same as today's behavior on the timeout path.
- **`Arc<TcpMessageBus>` keeps `signals_send` alive** while the cleanup
  thread holds its `Arc` clone, so the `Err(_)` branch on `signal_recv`
  is theoretical (defensive only).

## Out of scope

- Async side already uses notify; nothing to change.
- `TWS_READ_TIMEOUT` 1s socket-read poll on the dispatcher — separate
  concern (controls dispatcher shutdown latency, not cleanup), filed
  issue is specifically about the cleanup thread.
- Switching the signal channel to bounded for backpressure.
- v2-stable backport — issue is filed against main only; per memory
  `feedback_dual_branch_default` this is an ergonomic/test-perf fix not
  a bug, so no automatic v2-stable PR. Confirm with maintainer if
  unsure.
