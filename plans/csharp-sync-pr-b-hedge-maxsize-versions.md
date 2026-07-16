# PR-B — hedge_max_size + server-version constants + max bump

Part of the TWS 10.47.01 C# reference sync.

> **Blocked on PR-D (fundamentals decision).** Regenerating `protobuf.rs` drops the 3 fundamentals
> structs that `src/fundamental/` depends on, so the build won't compile until PR-D resolves how we
> vendor or remove them. Sequence PR-D (or at least its proto-retention mechanism) first.

## Background
Upstream server version is 225; we advertise 221 (`src/connection/common.rs:172`). The only proto
delta is `Order.hedge_max_size` (tag 144, int32 optional) added for HEDGE_MAX_SIZE (223, v10.45).
The recent order params at 216/217 are already plumbed in our proto encoder.

## Changes
1. Regenerate proto: `cargo run -p proto-gen` → adds `Order.hedge_max_size` (tag 144). Confirm the
   diff is exactly +`hedge_max_size` (± the fundamentals structs handled by PR-D).
2. Surface `hedge_max_size` on the public `Order` struct (`src/orders/`) + map it in
   `src/proto/encoders.rs` (Order → proto::Order), mirroring the sibling optional-int fields.
3. Add `src/server_versions.rs` constants:
   - `ADDITIONAL_ORDER_PARAMS_1 = 216`
   - `ADDITIONAL_ORDER_PARAMS_2 = 217`
   - `FRACTIONAL_LAST_SIZE = 222`
   - `HEDGE_MAX_SIZE = 223`
   - `USE_PRECISION_FROM_SEC_DEF = 224`
   - `ODD_LOT_BID_ASK_QUOTES = 225`
4. Bump advertised max in `src/connection/common.rs:172`: `UPDATE_CONFIG` (221) → `ODD_LOT_BID_ASK_QUOTES`
   (225). Update handshake version-range tests (they assert the `v{min}..{max}` string).
5. Optional but recommended: mirror C# `ValidateOrderParameters` — reject `hedge_max_size` (and the
   216/217 fields) with a clear error when `server_version <` the gate, instead of sending fields the
   server can't parse.

## Notes
- 222 (FRACTIONAL_LAST_SIZE) and 224 (USE_PRECISION_FROM_SEC_DEF) are pure version markers — no
  proto/decoder change; the data rides existing proto fields.
- 225 (odd-lot) tick-type decoding is PR-A's scope; this PR only adds its version constant + the bump.

## Verification
- Unit test for `hedge_max_size` encoding: drive the real order-place path, assert captured proto
  bytes (rule 10 — no builder→encode→decode self-loop).
- Version-constant tests derive assertions from the constant (rule 21).
- `CHANGELOG.md` `Added`: `Order.hedge_max_size`; server-version support through 225.
- Migration guide if the public `Order` shape changes.

## Checks
`cargo fmt`; all 3 clippy configs; all 3 rustdoc configs; `just test`; sync + async + all-features;
integration crates (`cargo build -p ibapi-integration-{sync,async} --tests`) since this touches
proto encoders.
