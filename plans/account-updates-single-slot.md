# account_updates: one account at a time — #847

**Implemented** on `account-updates-single-slot`. Deviations: sync
`account_updates` builds with `Subscription::new(..)` (mirrors the async
`new_from_internal`, no builder trait import); the `request_helpers`
`shared_request` pair lost its only caller and was deleted; `SharedCounts`
unit tests live in `src/transport/tests.rs`.

TWS `reqAccountUpdates` has one slot. Subscribing account B switches the feed
away from A; `A`'s handle keeps streaming B's frames. Since #836 (per-type
shared counts) dropping B withholds the cancel, so A streams B's data
indefinitely.

Fix per the issue: a second, different account while one is live is refused
with a typed error, no wire write. Same account shares, as today.

## 1. Transport: account slot on `SharedCounts` (`src/transport/mod.rs`)

`RequestAccountData` is the only request type with a single server-side slot
keyed by an argument, so model exactly that — no generic key map:

```rust
pub(crate) struct SharedCounts {
    generation: u64,
    live: HashMap<OutgoingMessages, usize>,
    /// Account of the live `RequestAccountData` subscriptions; `Some` iff
    /// `live[RequestAccountData] > 0`.
    account_updates: Option<AccountId>, // new
}
```

- `check_account_updates(&self, account) -> Result<(), Error>` — under the
  lock, **before** the write. `Err(AccountUpdatesInUse { active, requested })`
  when a different account is recorded.
- `subscribe_account_updates(&mut self, account) -> SharedTicket` — after a
  successful write: `subscribe(RequestAccountData)` + record the account.
- `unsubscribe` — when `RequestAccountData` reaches 0, set `account_updates = None`.
- `reset` — clears it with `live`.
- `#[cfg(test)] fn account_updates(&self) -> Option<&AccountId>` beside `live()`.
- Extend the type docs: one paragraph on the account slot.

A stale-generation ticket is already ignored, so a pre-reset A handle
dropping cannot clear a post-reset B.

## 2. Plumbing

The one caller gets one bus method; no builder / `Client` / request-helper
layer. `send_shared_request` and its 68 call sites are untouched.

- `MessageBus` + `AsyncMessageBus`:
  `send_account_updates_request(account: &AccountId, message)`. No
  `message_type` param: it is always `RequestAccountData`.
- Sync `TcpMessageBus`: extract the body of `send_shared_request` into a
  private `send_shared(message_type, account: Option<&AccountId>, message)`
  so queue registration/unregistration stays in one place.
  `SharedChannels::subscribe` gains the same `Option<&AccountId>` and runs
  check → write → subscribe under its one lock. The existing
  `Err → shared_channels.remove(&sender)` already unregisters on reject.
- Async `AsyncTcpMessageBus`: same private split; check before
  `write_message` inside the `shared_counts` lock block. The `resubscribe`d
  receiver taken before the lock is just dropped on reject.
- Stubs (`src/stubs.rs`, sync + async): implement by delegating to their
  `send_shared_request` (record bytes as today). No key recording — the
  request bytes `assert_request` already checks are encoded from the same
  `account`.
- `account_updates` (sync): `self.message_bus.send_account_updates_request(..)`
  → `self.subscription::<AccountUpdate>().build(internal)`.
  Async: `Subscription::new_from_internal(internal, bus, None, None, context)`,
  as `SubscriptionBuilder::send_shared` does. Two lines per side beats a
  single-caller builder method.

## 3. Error variant (`src/errors.rs`)

```rust
/// `account_updates` for a different account while one is live. TWS keeps
/// one account-updates subscription per connection.
#[error("account updates already streaming {active}; cancel it before requesting {requested}")]
AccountUpdatesInUse { active: AccountId, requested: AccountId },
```

`Error` is `#[non_exhaustive]` → additive. Add the arm to the manual
clone impl (~line 300) and the Display table in `errors_tests.rs`.
`AccountId` is `Clone + Display` (`src/accounts/types.rs`). Typed fields let
a caller match and fall back to `account_updates_multi(Some(&requested), None)`.

## 4. Release timing — async drop races

- Sync: `Drop` calls `cancel()`, which releases the count synchronously.
  `drop(a); client.account_updates(&b)` is sound.
- Async: `Drop` **spawns** the cancel. `drop(a); client.account_updates(&b).await`
  can run before the spawned task and fail with `AccountUpdatesInUse`.
  `a.cancel().await` releases before returning.

Handle in docs, not code: async rustdoc says to `cancel().await` every handle
before switching accounts; dropping alone releases the slot "shortly". Async
tests switch accounts via `cancel().await`, never via drop-then-subscribe (flaky).

## 5. Accounts rustdoc (`src/accounts/{sync,async}.rs`)

Sync + async (async lacks the one-account note today —
[doc parity](../docs/rules/docs/doc-parity-audit.md)):

- One account per connection. The same account again shares the stream; a
  different one returns `Error::AccountUpdatesInUse` until every handle for
  the first is cancelled (sync: or dropped; async: see §4).
- Concurrent accounts → `account_updates_multi` (request-id routed, no slot).
- `# Errors` section naming the variant.

Also fix `account_updates_multi` rustdoc (`src/accounts/sync.rs:237`): it
copies "Only one account can be subscribed at a time", the opposite of why
it exists. Check the async twin.

## 6. Tests

`SharedCounts` unit tests (sibling test file for `transport/mod.rs`, per
[sibling test files](../docs/rules/testing/sibling-test-files.md)): the account
slot's lifecycle — check/subscribe/unsubscribe-to-0/reset, stale ticket.

Transport (`src/transport/{sync,async}_tests.rs`, MemoryStream, beside the
#836 count tests), both sides:

1. same account twice → both `Ok`, `live == 2`, two request frames.
2. different account while live → `Err(AccountUpdatesInUse { active: A, requested: B })`,
   no frame written, `live == 1`; sync: subscriber registration removed.
3. cancel A (count → 0) → cancel frame written, slot empty; B → `Ok`.
4. reset with A live → B accepted; stale A drop neither cancels nor clears B.

Accounts tests: existing `test_account_updates` (sync + async) stay green via
the stub; no new stub infra.

Integration: no live test (paper = one account). Build
`ibapi-integration-{sync,async}` — bus trait changed
([integration crate builds](../docs/rules/workflow/integration-crate-builds.md)).

## 7. Changelog

`## [Unreleased]`:
- **Added**: `Error::AccountUpdatesInUse`.
- **Changed**: `account_updates` for a second account while one is live
  returns `Error::AccountUpdatesInUse` instead of silently switching every
  live handle to the new account's data (#847).

`examples/sync/account_updates.rs` is single-account; no README / migration
change.

## Rejected

From the issue:
- Per-account counts: TWS still has one slot; A starves silently.
- Per-subscriber filtering: `AccountUpdateTime` carries no account.
- Supersede A with an error: mirrors TWS but fails at a distance.

From lens review:
- Generic `key: Option<&str>` on shared requests + `keys` map: one consumer,
  and the transport would still build an accounts error — generic in name
  only. Two maps also duplicated the "key iff live > 0" invariant.
- `SharedRequestBuilder::keyed()` + `Client::send_keyed_shared_request` +
  request helper: three layers × two sides for one caller.
- Stub key recording: test infra with no signal beyond the request bytes.
- Making async drop release synchronously: `Drop` can't await the lock
  ([no block_on](../docs/rules/parity/no-block-on.md)).

## Open questions

- Account match is exact; don't normalize case.
- Same-account resubscribe writes a second request, and TWS replays the full
  snapshot to the existing handle too. Pre-existing; out of scope.

## Gate

Full pre-PR list in `CLAUDE.md` + `cargo build -p ibapi-integration-{sync,async} --tests`
+ `just rules-check` (plan file).
