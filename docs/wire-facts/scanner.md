# Scanner wire facts

Scanner subscriptions and their data rows. Format: [README](README.md).

### `distance`, `benchmark` and `projection` are absent

*Live 2026-11-15 20:30 ET weekend (Sunday evening), paper, server 226; #1234.*

- `reqScannerSubscription`, 10 rows each:
  - stocks (`STK`, `STK.US.MAJOR`): `MOST_ACTIVE`, `TOP_PERC_GAIN`, `HIGH_OPT_IMP_VOLAT`
  - bonds (`BOND`, `BOND.US`): `HIGH_BOND_ASK_YIELD_ALL`, `FAR_MATURITY_DATE`
- In all 50 `ScannerDataElement`s, proto tags 4 (`distance`), 5 (`benchmark`) and 6
  (`projection`) were absent. Stock rows were read through the decoder; bond rows were decoded
  from a raw capture. `combo_key` was absent too.
- This matches the proto comments ("Internal use only", "Always returns an empty string"). The
  C# client still forwards all three to `EWrapper.scannerData`, documented only as "according
  to query".
- **Not probed:** futures scans (`FUT.US` `MOST_ACTIVE` drew 492, missing market-data
  permissions), options and combo/EFP scans, scans with filter tag-values, RTH.

### Bond scans send 165 before their rows

*Live 2026-11-15 20:30 ET weekend (Sunday evening), paper, server 226.*

- Both bond scans above sent error 165 "Historical Market Data Service query message:10 out of
  18957 items retrieved" on the request id, then the `ScannerData` frame with all 10 rows.
- The crate treats this 165 text as an error, so the subscription ends with `Err` and the rows
  that follow are dropped. Only the "no items retrieved" text is handled as informational.
  Stock scans sent no 165.
- **Not probed:** whether later refreshes of a bond scan repeat the 165, other instruments, RTH.
