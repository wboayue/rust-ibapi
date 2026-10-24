---
id: timezone-free-timestamps
title: Send and parse timestamps in fixed, zone-free forms; pass zoned text through
cluster: wire
status: active
triggers:
  - adding a request option that changes how TWS renders a date or time
  - parsing a date or time string from the wire
  - reaching for find_timezone, resolve_local, time_tz, or a zone name lookup
  - assuming the gateway's, TWS login's, or host's time zone
  - formatting an OffsetDateTime for an outbound request
symbols: [find_timezone, resolve_local, parse_connection_time, parse_historical_data_end_timestamp, parse_schedule_date_time, parse_time_as_utc, ToField, OffsetDateTime, formatDate]
related: [enum-typing, proto-only-decoding]
precedents: ["#808", "#904", "#1148"]
memory: []
---

Don't add code whose result depends on knowing a time zone: no zone-name lookup, no
resolving a local wall clock to an instant, no DST handling, no assumed gateway, TWS-login or
host zone. Use a form that's the same everywhere, or hand the text to the caller unchanged.

- **Outbound:** take an `OffsetDateTime`, convert it to UTC and send the fixed
  `YYYYMMDD HH:MM:SS UTC` rendering (`ToField for OffsetDateTime` in
  `market_data/historical/common/encoders.rs`, `manual_order_time` in
  `orders/common/encoders.rs`). A zone the caller names for TWS to apply (an algo's
  `active_time_tz`) passes through as their string. The crate doesn't interpret it.
- **Inbound, prefer a zone-free wire form:** Unix seconds (`formatDate=2`), a bare date
  (`YYYYMMDD` → `Date`), or a rendering that is UTC by definition (`YYYYMMDD-HH:MM:SS`, the
  gateway's UTC format). Parse it with one `format_description!`, not a list of guesses. A
  zone-less text is UTC only if a doc or a live capture says so (`docs/wire-facts/`), as for
  historical news `time` (`news::common::decoders::parse_time_as_utc`, see
  [news wire facts](../../wire-facts/news.md#historical-news-time-is-utc)).
- **Inbound, only zoned or local text available:** pass it through as a `String` field. Don't
  resolve it. Leave the zone to the caller, who knows their gateway setup.
- **Don't add request options that switch TWS to a zone-dependent rendering.** `formatDate=1`
  makes bar timestamps follow the gateway's Historical Date Format setting. #1148 was declined
  because epoch seconds already give the caller exact instants.

## Existing exceptions

These were written before this rule and resolve a zone. Don't extend them, don't
copy them, and don't route a new field through them:

- `connection::common::parse_connection_time`: the handshake's `YYYYMMDD HH:MM:SS <zone>`,
  which also sets `Client::time_zone()`.
- `parse_historical_data_end_timestamp` (`market_data/historical/common/decoders.rs`): the two
  `HistoricalDataEnd` shapes captured on a live gateway (#808). An unknown shape fails loudly
  rather than being guessed (#904).
- `parse_schedule_date_time`: historical schedule sessions, resolved in the schedule's own
  `time_zone`.

A new decoder that seems to need one of these is a design question. Raise it before writing
the code.

## Why

A zone-dependent result changes with things the crate can't see: the gateway's date-format
setting, the zone chosen at TWS login, the host's tz database, and Windows-only zone names.
When it's wrong, it's *silently* wrong: an instant off by hours, with no error. DST adds a
repeated hour and a skipped hour that need a policy to resolve. A fixed form is exact or fails
loudly, and zoned text passed through is still the caller's call.

## Precedents

- #808 proposed six `HistoricalDataEnd` timestamp shapes. Review narrowed it to the two
  captured live, because a zone-less wall clock resolved in a guessed zone is a wrong instant.
- #904 tracks candidate shapes that are accepted only with a raw capture.
- #1148 (declined): `DateFormat::Text` for historical bars. It would have made
  `BarTimestamp::from_str` resolve zone names and DST, depending on the gateway's setting,
  without changing what a caller gets. SPX daily closes already decode as
  `BarTimestamp::Date` and intraday bars as exact UTC instants under `formatDate=2`.
