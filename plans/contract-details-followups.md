# Contract details stream — follow-ups (#876)

#876 shipped in #878 (builder, decoder, Vec rewrite, bond fix) and #879
(`buffer_limit`, `cancel_and_drain`). Plans are in those PRs' history.

## Open

- **Id before I/O on other builders** (e.g. `option_chain(..).request_id()`,
  allocation moved into `OptionChainBuilder::new`). Same change each time; add
  per builder when a caller needs it, not as a sweep.
- **`buffer_limit` on other builders** (option chain, historical data), when a
  caller asks.
- **`next_request_id()` is public.** Do the builder docs need a note that ids
  from it can't be handed to this builder (no setter)?

## Wire notes (live, 2026-09-30, server 225)

- `cancelContractData` doesn't stop a prepared result: SPY 202611 calls (714
  rows), cancel after 3 rows, TWS still sent 711 rows over ~1.1s, then
  `ContractDataEnd`.
- Unfiltered queries (all SPY options) arrive incrementally: first row 8.4s,
  4,340 rows by ~113s, then 60s with no end marker. A cancel doesn't stop them
  either (100 rows, cancel, 8,316 more over 150s). Four such runs killed
  mid-stream wedged the gateway until restart: keep unfiltered queries out of CI.
