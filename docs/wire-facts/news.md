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
- Settles `news::common::decoders::parse_time_as_utc`'s UTC assumption: it is correct.
- **Not probed:** a gateway logged in under a non-US zone, or set to the UTC Historical Date
  Format (the article time may or may not follow that setting); `tickNews` (epoch ms, not
  affected); a missing or unparseable `time` value, which the decoder now returns as
  `Error::Parse`.
