# TWS API release-notes coverage (prod-2026)

Cross-reference of every API item in <https://www.ibkrguides.com/releasenotes/prod-2026.htm>
against this crate. Reviewed 2026-07-16 (release notes span TWS 10.43 → 10.48).

| TWS ver | Release-notes item | Status | Where |
|---|---|---|---|
| 10.48 | `reqOpenOrders` now returns de-activated orders | ✅ Shipped (`Order.deactivate`) | See note 1 |
| 10.47 | Fundamental data removed (`reqFundamentalData`/`cancelFundamentalData`, callbacks, `FUNDAMENTAL_RATIOS` 47) | ✅ Shipped | PR #705 |
| 10.47 | `$LEDGER-` prefix on per-currency account values (`reqAccountUpdates{,Multi}`, `updateAccountValue`) | ⚠️ Verify — likely no-op | See note 2 |
| 10.46 | Odd-lot tick *types* 105–110 (`oddLotBid`…`oddLotAskExch`) | ✅ Shipped | PR #703 |
| 10.46 | Generic tick **787** (odd-lot request-side) | ✅ Shipped | PR #706 |
| 10.45 | `hedgeMaxSize` order param (proto tag 144) | ✅ Shipped | PR #706 |
| 10.44 | update-config request/response | ✅ Shipped | PR #708 |
| 10.44 | Fractional last sizes in `tickSize` (`FRACTIONAL_LAST_SIZE` 222) | ✅ Shipped (version marker) | PR #706 |
| 10.44 | `BarSize::Min4` (4-min bar) | ✅ Shipped | PR #704 |
| 10.43 | get-config request/response | ✅ Shipped | PR #707 |
| 10.43 | Mobile app order submission (stage via API, submit via mobile) | ✅ No API surface — no-op | See note 3 |

## Legend
- ✅ Shipped / genuine no-op · 📋 Covered by an open plan · ⚠️ Needs a verify pass · ❌ Uncovered gap

## Open work
- None. `Order.deactivate` (the last gap) shipped; only advisory verify note 2 (`$LEDGER-` doc note)
  remains, and it needs no library change.

## Verify notes
1. **De-activated open orders (10.48):** server-behavior change — `reqOpenOrders` now *includes*
   orders that were previously filtered out. **Shipped:** `Order.deactivate: bool` on the public
   `Order`, mapped in both directions in `src/proto/{encoders,decoders}.rs` (encode via `some_bool`,
   decode via `unwrap_or_default`), with encode/decode unit tests and a `CHANGELOG.md` `Added` entry.
   No server-version gate (`Order.Deactivate` predates the proto floor).
2. **`$LEDGER-` account-value prefix (10.47):** a TWS *setting* that prepends `$LEDGER-` to
   per-currency account-value **keys**. Our account-value decoding is keyed on arbitrary strings, so
   the wire still parses; downstream code that string-matches specific keys (e.g. `"CashBalance"`)
   could miss prefixed variants. **Action:** none in the library; consider a troubleshooting-doc note
   so users know the key can carry the prefix.
3. **Mobile order submission (10.43):** a TWS/mobile capability (stage an order via API, release it
   from the mobile app). No new API request/field on the client side — genuine no-op.

## Re-run
Re-fetch the release-notes page and diff this table when bumping the C# reference or the advertised
server version. New rows start as ❌/⚠️ until a plan claims them.
