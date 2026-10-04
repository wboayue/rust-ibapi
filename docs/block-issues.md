# Blocked issues

Work that can't move until something outside the code changes: a capture from a gateway in a
particular configuration, or account setup on the paper account. Each entry says what unblocks it.
When that happens, open a GitHub issue from the entry (or do the work directly) and delete the
entry here.

Moved from GitHub issues labeled `blocked` on 2026-10-04. The original issue numbers are kept so
links from commits, PRs and code comments still lead somewhere.

- [HistoricalDataEnd timestamp renderings](#historicaldataend-timestamp-renderings-904) (#904): needs a raw capture of a new rendering
- [Preset attached orders: child lifecycle](#preset-attached-orders-child-lifecycle-905) (#905): needs order presets defined on the paper account

## HistoricalDataEnd timestamp renderings (#904)

**Unblocked by:** a raw capture (`IBAPI_RAW_CAPTURE_DIR`) of a `HistoricalDataEnd` timestamp in a
shape below, with the gateway version and API date/time setting that produced it.

`parse_historical_data_end_timestamp` (`src/market_data/historical/common/decoders/mod.rs`) accepts exactly the shapes captured on a 213+ gateway. #808 first proposed six; review narrowed it to two, because a zone-less wall clock resolved in a guessed zone is a silently wrong instant, while an unknown shape is a loud decode error that a capture can fix.

### Verified (accepted)

| Rendering | Source | Resolution |
| --- | --- | --- |
| `20260101 09:30:00 US/Eastern` | default "instrument timezone" setting; fixtures since 2.x | wall clock in the named zone; the zone is everything after the time (`China Standard Time` works) |
| `20260910-04:33:56` | IB Gateway 10.50, "Send instrument-specific attributes ... in: UTC format", raw capture in #808 | UTC |

### Candidates (rejected today: parse error)

Promote one only with a capture (`IBAPI_RAW_CAPTURE_DIR`), recording the gateway version and the API date/time setting that produced it.

| Rendering | Why it was proposed | What it would need |
| --- | --- | --- |
| `20260101 09:30:00` (zone-less) | ib_insync `parseIBDatetime` tolerates it; pre-10.17 TWS | capture, and which zone it means (instrument? gateway host? UTC?). Most likely to be wrong if assumed UTC |
| `20260101  09:30:00 ...` (two spaces) | ib_insync e286b0b; old text `historicalDataEnd` | capture on protobuf transport; if seen, collapse whitespace rather than special-case |
| `2026-01-01 09:30:00[.0]` | news `try_parse_time_as_utc` parses this on `NewsArticle`; ib_async#230 | capture; if seen, share the parser with `src/news/common/decoders.rs` |
| `2026-01-01 09:30:00 UTC` | dashed date + zone | capture |

### Consolidation, when triggered

- If a dashed-date shape is verified, fold `news::common::decoders::try_parse_time_as_utc` and this parser into one helper under `common::timezone` (review item 6 on #808).
- `parse_schedule_date_time` shares `DASHED_DATE_TIME` with the UTC rendering; keep that single constant if the format list grows.
- `YYYYMMDD HH:MM:SS <zone>` splitter (from #859): `parse_connection_time` (`src/connection/common.rs`, soft on an unknown zone) and `parse_historical_data_end_timestamp` (hard on every failure) both split with `splitn(3, ' ')`. Error policies differ, so a shared helper could only return the parts (date, time, zone text). Land it when a third parser of this rendering appears.

## Preset attached orders: child lifecycle (#905)

**Unblocked by:** stock order presets with an attached stop-loss and profit-taker defined for the
paper user (see "Unblock first").

Follow-up to #842 (shipped in #849). The encoding shipped, but the paper account had no order presets, so every attach ended in 10355 with the parent discarded, and these behaviors were never observed.

### Unblock first

Define stock presets with an attached stop-loss and profit-taker in TWS (Global Configuration → Presets) for the paper user, stored on the server so IB Gateway picks them up. Confirm with `preset_attached_orders_accepted` (sync or async): its printed outcome flips from `no preset` to `attached`.

### Open questions

1. **Child frames.** For the SL/PT ids: `OpenOrder` / `OrderStatus` shape, `order.parent_id`, the `order_type` / prices TWS chose, ordering relative to the parent, and whether they arrive at placement (held) or only after the parent fills. Also any `OrderBound` frames for the child ids.
2. **`transmit = false`.** Are the children created held with the parent, and does a later re-send of the parent with `transmit = true` release them? A re-send after a discard gets 103 (duplicate order id), so test against a *working* held parent.
3. **Parent cancel.** Do the children cancel with the parent (OCA-like), and which frames/notices arrive for each id?
4. **Modify.** Re-sending the parent (same id, new limit) with the same preset ids: does TWS keep, re-create, or reject the children? Without the preset ids?

### Decisions that hang on the answers

- **Routing.** Shipped as option A: child frames reach only `order_update_stream`. If (1) shows callers need the family on one subscription, option B registers the child ids onto the parent's order channel in `send_order_request` (both transports; run the integration crate builds).
- **Integration test.** Tighten `preset_attached_orders_accepted` to require `attached`, assert both children's `parent_id`, and add a parent-cancel assertion from (3).
- **Docs.** `Order::preset_*_order_id`, `OrderBuilder::preset_*`, `docs/order-types.md` and `examples/async/preset_attached_orders.rs` describe only the no-preset failure; add the observed child lifecycle and `transmit` behavior.

### Harness

Rebuild the probe as a temporary `#[ignore]`d in-crate async test: open `order_update_stream()`, place a far-from-market parent via `.preset_stop_loss().preset_profit_taker().submit()`, print every item for ~6 s, cancel the parent, print again, then cancel the child ids.
