# News wire facts

News providers, historical news, articles. Format: [README](README.md).

### Historical news time is UTC

*Live 2026-11-04 00:06 ET overnight, paper, server 226; gateway login zone US/Eastern (-05:00).*

- `reqHistoricalNews` for AAPL (265598) and SPY (756733), all 8 paper providers, last 2 days,
  10 results each. `HistoricalNews.time` arrives as zone-less text `YYYY-MM-DD HH:MM:SS.0`
  (raw capture: `2026-11-04 02:30:00.0`).
- The text is UTC, not the gateway's zone:
  - The newest AAPL item, `2026-11-04 02:30:00.0`, read as US/Eastern would be 2h24m in the
    future (it was 00:06 ET). Read as UTC it was 2h36m old.
  - DJ-N's automated "SPY ETF Declines/Gains n%" close reports carry `20:36`–`20:37` on August
    days. As UTC that's 16:36 EDT, just after the close. Every zone east of UTC puts them
    before the close.
- Settles `news::common::decoders::try_parse_time_as_utc`'s UTC assumption: it is correct.
- **Not probed:** a gateway logged in under a non-US zone, or set to the UTC Historical Date
  Format (the article time may or may not follow that setting); `tickNews` (epoch ms, not
  affected); a missing or unparseable `time`. The decoder leaves such an article's `time` as
  `None`, logging a warning for unparseable text (#1154).

### Historical news total_results range

*Live 2026-11-05 23:41 ET overnight, paper, server 226; #1190.*

- `reqHistoricalNews` for AAPL (265598), all 8 paper providers, last 90 days (more than 300
  headlines in range), one request per `totalResults` value.
- 1, 10 and 300 return that many headlines. 301, 500, 1000 and 65535 each return 300, the
  same newest-first 300, ending normally with no notice or error.
- 0 is rejected: error 321 "Error validating request.-'Z' : cause - Total results must be > 0".
- Settles IB's documented range of 1 – 300: the upper bound is a silent cap, the lower bound
  an error.
- **Not probed:** negative values (the public parameter is `u16`, so the crate can't send
  them); a range holding fewer than the requested headlines.

### Historical news range and has_more

*Live 2026-11-14 11:45–12:30 ET Saturday, paper, server 226; gateway login zone US/Eastern. #1212.*

- `reqHistoricalNews` for AAPL (265598), all 8 paper providers, `totalResults` 300 unless noted,
  with `startDateTime` / `endDateTime` set as raw text (empty = omitted).
- **Format.** The historical-data form `yyyyMMdd HH:mm:ss UTC`, which the crate sent before
  #1212, is ignored without a notice: every request returned the newest headlines, so a 30-minute
  window returned 300 headlines spanning 13 days. `yyyyMMdd HH:mm:ss` with no zone is ignored too.
  The C# reference's `yyyy-MM-dd HH:mm:ss.0` is read as **UTC**: start `2026-11-04 12:00:00.0`
  returned newest `11:36:08` UTC, and AAPL had headlines throughout the hours an Eastern
  reading (17:00 UTC) would have included. With ` UTC` appended it reads the same. Without the
  `.0`, `2026-11-04 12:00:00` is read in the **gateway's zone**: newest `16:02` UTC, before
  12:00 ET = 17:00 UTC.
- **One bound per request.**
  - `startDateTime` alone: the newest headlines strictly **before** it (exclusive: start
    `13:15:00.0` excludes a `13:15:00` headline, `13:15:01.0` includes it).
  - `endDateTime` alone: the newest headlines **at or after** it (inclusive: end `00:01:00.0`
    includes a `00:01:00` headline). End Nov 13 00:00 returned 20 headlines, `has_more` false.
  - Both set: `endDateTime` is ignored. Start Nov 1 with end Nov 5, and with end now, returned
    the same page; start Nov 4 14:00 with end Nov 4 12:00 returned 300 headlines back to Oct 20.
  - Neither: the newest headlines.
- **`has_more`** is `true` when more headlines match than were returned (`totalResults` 1, 10,
  300, and 500 capped to 300) and `false` when all fit (the 20 since Nov 13).
- **Not probed:** non-US gateway zones and the gateway's UTC Historical Date Format setting;
  headlines sharing the timestamp of a `startDateTime` page boundary (skipped by an exclusive
  bound; timestamps are mostly whole minutes); `historicalNewsOptions`.
