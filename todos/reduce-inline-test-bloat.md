# Reduce inline test bloat in src/

## Problem

Inline `#[cfg(test)]` blocks total **~29,990 lines** across `src/` — roughly 45% of the source tree. Every `Read`/`Grep` on these files pulls test code into context alongside the logic being edited, inflating token usage and slowing navigation.

Worst offenders:

| File | code lines | inline test lines | test % |
|---|---|---|---|
| `src/client/sync.rs` | 19 | 3046 | 99% |
| `src/client/async.rs` | 326 | 2606 | 89% |
| `src/messages.rs` | 23 | 1376 | 98% |
| `src/client/common.rs` | 0 | 1287 | 100% |
| `src/orders/builder/order_builder.rs` | 7 | 1039 | 99% |
| `src/orders/common/decoders.rs` | 1206 | 1306 | 52% |
| `src/accounts/common/decoders.rs` | 381 | 1070 | 74% |

Splitting the top 6 alone reclaims ~10k+ lines from routine reads.

## Precedent

`src/orders/builder/order_builder/tests.rs` (1313 lines) is already split from `order_builder.rs`. Same pattern extends to everything else.

## Plan

### Phase 1 — Split inline tests into sibling `tests.rs` files (biggest win)

Mechanical refactor for each target file `foo.rs`:

1. Create `foo/tests.rs` (or `foo/mod.rs` if `foo.rs` needs to become a directory)
2. Move the entire `#[cfg(test)] mod tests { ... }` block verbatim
3. In `foo.rs`, replace with:
   ```rust
   #[cfg(test)]
   mod tests;
   ```
4. Fix `use super::*;` paths as needed (usually unchanged)
5. Run `cargo test` + clippy (all feature configs) to verify zero behavioral change

Targets (in priority order):

- [ ] `src/client/sync.rs` → `src/client/sync/tests.rs`
- [ ] `src/client/async.rs` → `src/client/async_/tests.rs`
- [ ] `src/messages.rs` → `src/messages/tests.rs` (messages/tests.rs already exists at 1731 lines — check for conflict; may already be partially split)
- [ ] `src/orders/common/decoders.rs` → `src/orders/common/decoders/tests.rs`
- [ ] `src/accounts/common/decoders.rs` → `src/accounts/common/decoders/tests.rs`
- [ ] `src/orders/builder/order_builder.rs` — verify whether the 1039 inline lines are distinct from the existing `order_builder/tests.rs` (may be a doctest or leftover block)

Secondary targets (200+ inline test lines each) — batch after primary 6 land:
`market_data/historical/{sync,async}.rs`, `accounts/{sync,async}.rs`, `orders/{sync,async}.rs`, `transport/{sync,async}.rs`, `market_data/realtime/**`, `contracts/**`, `client/error_handler.rs`, etc. See `git grep -c '#\[cfg(test)\]'` ranking for the full list.

### Phase 2 — Rename `src/client/common.rs` → `client/test_support.rs`

That file is 100% `#[cfg(test)] pub mod mocks` with **zero** production code. Its current name implies shared production code, forcing re-reads to confirm.

- [ ] Move to `src/client/test_support.rs` (or under `src/stubs/`)
- [ ] Update `mod` declaration in `src/client/mod.rs`
- [ ] Update imports across the codebase (likely `use crate::client::common::tests::*;` → `use crate::client::test_support::mocks::*;`)

### Phase 3 — Extract large fixture tables to data files

Large table-driven test vectors live in:

- `src/accounts/common/test_tables.rs`
- `src/contracts/common/test_tables.rs`
- `src/market_data/historical/common/test_tables.rs`
- (others — grep for `test_tables`)

Plan:

- [ ] Identify which are genuinely large (>500 lines of literal data)
- [ ] Move the data to `tests/data/*.json` (directory already exists)
- [ ] Load via `include_str!` + `serde_json::from_str`
- [ ] Leave small tables inline — not worth the ceremony

### Phase 4 — Audit oversized rustdoc doctests (optional)

After Phase 1, check whether the remaining non-test portion of `client/sync.rs` / `async.rs` is still inflated by large rustdoc code blocks. If so, move the longer examples to `examples/` and link from the doc comment.

## Verification per phase

For each file moved:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
```

## Non-goals

- No test behavior changes
- No test additions or deletions
- No restructuring of test helpers beyond the `common.rs` rename
- No changes to `tests/` (integration tests)

## Expected outcome

- `Read` on `client/sync.rs` drops from ~3065 lines to ~20
- `Read` on `messages.rs` drops from ~1399 lines to ~25
- Overall ~30k lines removed from the hot read path without losing a single test
- Clearer separation of production code vs. test scaffolding
