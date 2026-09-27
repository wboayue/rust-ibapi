# Timezone follow-ups (#809)

Deferred from the lens review of PR #859, which made zone lookup exact
(`get_by_name` + `TIMEZONE_ALIASES`) and made an unrecognized handshake zone a
warning instead of a connect failure. Historical decoding (`HistoricalDataEnd`,
`Schedule`) still fails with `Error::UnsupportedTimeZone`.

## 1. Remove `DecoderContext.time_zone`

`src/subscriptions/common.rs`. No decoder reads it: the text decoders that used
it went with the protobuf-only transport (#632). Both clients still populate it
from `Client::time_zone` in `decoder_context()`.

- Drop the field and the `time_zone` parameter of `DecoderContext::new`
  (22 call sites, mostly tests) and the struct literals in
  `src/subscriptions/common_tests.rs`.
- Crate-internal (`pub(crate) use` in `subscriptions/mod.rs`): no changelog or
  migration entry.
- Leaves `Client::time_zone` as the handshake zone's only consumer, with
  `connection_time`.

Restructuring, not mechanical: own PR.

## 2. Shared `YYYYMMDD HH:MM:SS <zone>` splitter (rule of three)

Two parsers read this rendering:

- `parse_connection_time` (`src/connection/common.rs`) — handshake; soft on an
  unknown zone, keeps the zone when the date fails to parse.
- `parse_historical_data_end_timestamp`
  (`src/market_data/historical/common/decoders/mod.rs`) — hard on every failure;
  also accepts the zone-less UTC rendering.

Both now split with `splitn(3, ' ')` so a multi-word zone is the remainder. The
error policies differ, so a shared helper could only return the parts (date,
time, zone text) and leave resolution to the caller. Two occurrences don't pay
for it; land it when a third parser of this rendering appears.
