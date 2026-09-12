# HistoricalDataEnd start/end renderings: verified vs. candidate (#808)

`parse_historical_data_end_timestamp` in
`src/market_data/historical/common/decoders/mod.rs` accepts exactly the shapes
that have been captured on a 213+ gateway. PR #808 originally proposed six
shapes; review narrowed it to two because a zone-less wall clock resolved in a
guessed zone is a silently wrong instant, whereas an unknown shape is a loud
decode error that a capture can fix in an afternoon.

## Verified (accepted)

| Rendering | Source | Resolution |
| --- | --- | --- |
| `20260101 09:30:00 US/Eastern` | default "instrument timezone" setting; fixtures since 2.x | wall clock in the named zone; zone is everything after the time (`China Standard Time` works) |
| `20260910-04:33:56` | IB Gateway 10.50, "Send instrument-specific attributes ... in: UTC format", raw capture in #808 discussion | UTC |

## Candidates (rejected today, parse error)

Each of these was in #808's first revision on the strength of docs, an
adjacent message, or another client's history. None has a capture on this
message from a 213+ gateway. Promote one only with a capture
(`IBAPI_RAW_CAPTURE_DIR`), and record the gateway version and the API date/time
setting that produced it.

| Rendering | Why it was proposed | What it would need |
| --- | --- | --- |
| `20260101 09:30:00` (zone-less classic) | ib_insync `parseIBDatetime` tolerates it; pre-10.17 TWS behaviour | capture; and the zone it means (instrument? gateway host? UTC?) — this is the shape most likely to be wrong if assumed UTC |
| `20260101  09:30:00 ...` (two spaces) | ib_insync e286b0b; old `historicalDataEnd` text rendering | capture on protobuf transport; if seen, collapse whitespace rather than special-case |
| `2026-01-01 09:30:00[.0]` | news `try_parse_time_as_utc` parses this on `NewsArticle`; ib_async#230 | capture; if seen, share the parser with `src/news/common/decoders.rs` instead of a second copy |
| `2026-01-01 09:30:00 UTC` | dashed date + zone | capture |

## Follow-ups

- If a dashed-date shape is verified, fold `news::common::decoders::try_parse_time_as_utc`
  and this parser into one helper under `common::timezone` (review item #6 on #808).
- `parse_schedule_date_time` already shares `DASHED_DATE_TIME` with the UTC
  rendering; keep that single constant if the format list grows.
