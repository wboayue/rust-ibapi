# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Error::UnexpectedWireFormat`, returned when a proto-only decoder receives a text-framed message. Previously this shared `Error::UnexpectedResponse` with the unrelated "message is not for this decoder" case, which the dispatcher skips silently. `Error` is `#[non_exhaustive]`, so the new variant is not a breaking change (#731).

### Changed

- A text-framed message reaching a proto-only decoder now fails the subscription instead of being skipped. At `server_versions::PROTOBUF_REST_MESSAGES_3` every message with a proto decoder arrives proto-framed, so reaching this means the gateway broke protocol — previously the message was dropped and the subscription silently yielded nothing. Wrong-message-type frames are still skipped, which is what shared channels need (#731).
- Historical tick and histogram sizes are now `Option<f64>` instead of `i32`: `TickMidpoint.size`, `TickLast.size`, `TickBidAsk.size_bid`/`size_ask`, and `HistogramEntry.size`. IBKR models these as decimals on the wire, and the old `i32` parse silently truncated fractional sizes — a crypto tick of `0.5` decoded as `0`. `None` means TWS sent no value (field absent, empty, or an "unset" sentinel); `Some(0.0)` is a real zero. This also changes the serialized shape — a size is now `100.0` rather than `100`, absent is `null` rather than `0`, and the `utoipa` schema becomes a nullable `number` (#716).
- `ContractDetails.min_size`, `size_increment`, and `suggested_size_increment` are now `Option<f64>` instead of `f64`. Contracts without size rules omit these on the wire, where the old `0.0` was indistinguishable from a real value and a `size_increment` of `0.0` is nonsense (#716).

### Removed

- `Client::check_server_version()` is crate-private. It was `pub` on the async client and `pub(crate)` on the blocking one — drift rather than design, since it is the internal guard every version-gated API already calls. Compare against `Client::server_version()` directly if you need to branch on gateway support (#728).

### Fixed

- `TickTypes::MarketDataType` now reaches `Client::market_data` subscriptions. The message type was missing from the request-id routing allow-list, so TWS's market-data-type notifications (real-time / frozen / delayed / delayed-frozen, sent on subscribe and whenever the feed switches) were routed to a shared channel nobody subscribes to and dropped. The decoder has produced the variant since #516; nothing could ever yield it (#730).

- Decimal-typed wire fields no longer fall back to `0` when the value fails to parse; a malformed value now surfaces as `Error::Parse` and fails the request or subscription instead of being silently swallowed. Covers order quantities, execution shares, positions, contract-detail sizes, bar volume/WAP, tick and market-depth sizes (#716).
- TWS "unset" sentinels (`2147483647`, `9223372036854775807`, `-9223372036854775808`, `1.7976931348623157E308`) are now recognized on those same fields rather than only `OrderState.suggested_size` and `OrderAllocation.*`. They decode to `None`, or to `0.0` on fields still typed `f64`, instead of leaking as a literal 2.1-billion size (#716).

## [3.3.0] - 2026-07-16

### Added

- `Client::config()` (sync + async) to read the TWS/Gateway configuration (API settings, order precautions, smart-routing, lock-and-exit) added upstream for server version 219 (TWS 10.43); returns the new `config::Config` snapshot type and is gated behind `server_versions::CONFIG`.
- `Client::update_config()` (sync + async) to write partial edits to the TWS/Gateway configuration via a fluent `UpdateConfigBuilder` (`.api()`/`.orders()`/`.lock_and_exit()`/`.message()`/`.messages()`/`.accept_warning()`/`.accept_warnings()`/`.reset_api_order_sequence()` → `.submit()`), added upstream for server version 221 (TWS 10.44); returns the new `config::UpdateConfigResponse` (with `config::ConfigWarning`) and is gated behind `server_versions::UPDATE_CONFIG`.
- `Order.deactivate` (`bool`) flagging a de-activated (inactive) order; since TWS 10.48 `reqOpenOrders` returns de-activated orders, so this distinguishes them from active ones. Round-trips through the order proto in both directions; no server-version gate (#709).
- `Order.hedge_max_size` (`Option<i32>`) for the maximum size of a hedge order, added upstream for server version 223 (TWS 10.45); gated behind `server_versions::HEDGE_MAX_SIZE` when placing orders (#706).
- Server-version support advertised through 225 (`ODD_LOT_BID_ASK_QUOTES`), with new `server_versions` constants `FRACTIONAL_LAST_SIZE` (222), `HEDGE_MAX_SIZE` (223), `USE_PRECISION_FROM_SEC_DEF` (224), and `ODD_LOT_BID_ASK_QUOTES` (225) (#706).
- `generic_tick::ODD_LOT` (`"787"`) request-side generic tick to subscribe odd-lot bid/ask quotes (server 225 / TWS 10.46) via `reqMktData` (#706).
- `BarSize::Min4` (`"4 mins"`) historical bar size, added upstream in TWS API v10.44 (#704).
- Odd-lot bid/ask `TickType` variants (`OddLotBid`, `OddLotAsk`, `OddLotBidSize`, `OddLotAskSize`, `OddLotBidExch`, `OddLotAskExch`, ids 105–110) so odd-lot market-data ticks (server 225 / TWS 10.46) decode to typed variants instead of `Unknown` (#703).

### Removed

- Fundamental data support: the `fundamental` module (`FundamentalData`, `FundamentalReportType`) and `Client::fundamental_data`. IBKR removed the fundamental-data feature (`reqFundamentalData`/`cancelFundamentalData`) from the TWS API in 10.47 with no replacement. The `TickType::FundamentalRatios` variant (tick id 47) is also removed; id 47 now decodes to `TickType::Unknown`.

## [3.2.1] - 2026-07-13

### Fixed

- Request-less TWS errors on shared one-shot requests (e.g. read-only-mode 321, unknown market rule 322) now fail the awaiting call fast with the real error instead of hanging; streaming shared subscriptions are unaffected (#698).

## [3.2.0] - 2026-07-06

### Added

- `AggTrades` variant (wire value `AGGTRADES`) on the historical and realtime `WhatToShow` enums, required to request trade bars for crypto contracts (TWS rejects `TRADES` with error 10299) (#693).
- `ConnectivityStatus` enum with `ConnectivityStatus::from_code()` and `Notice::connectivity_status()` to expose data-farm connectivity sub-states (Ok / Broken / Inactive / Connecting) within the 2100–2169 warning band (#684).
- `Error::is_connection_lost()` predicate so reconnect loops can branch on connection loss without matching internal error variants (#690).
- `Subscription::collect_for(timeout)` / `collect_until(timeout, predicate)` terminals and `MarketDataBuilder::snapshot_once(timeout)` to collect a one-shot snapshot into a `Vec` without hand-writing a collect-with-timeout loop (#686).

### Fixed

- Connectivity restored notices no longer log at `error`: code 1102 ("data maintained") now logs at `info` and code 1101 ("data lost — resubscribe") at `warn`, so routine overnight reconnects stop tripping error-level alerting (#695).
- Async snapshot market-data subscriptions no longer send a redundant cancel after the snapshot completes, matching the sync side (#686).

## [3.1.0] - 2026-06-19

### Added

- `DATA_ADVISORY_CODES`, `Notice::is_data_advisory()`, and the `NoticeCategory::DataAdvisory` variant for delayed-market-data advisory codes (#680).

### Changed

- Benign data-farm connectivity notices (codes 2104/2106/2158, "…connection is OK") now log at `info` instead of `warn`, removing warn-level spam on long-running sessions (#678).

### Fixed

- Delayed-data advisories (codes 10089/10167) no longer terminate a market-data subscription before its data arrives (#677).
- `TickSubscription` now carries the `SubscriptionItem` envelope (#675).

## Prior releases

Versions up to and including [3.0.1] predate this changelog; see the
[GitHub Releases page](https://github.com/wboayue/rust-ibapi/releases) for their notes.

[Unreleased]: https://github.com/wboayue/rust-ibapi/compare/v3.3.0...HEAD
[3.3.0]: https://github.com/wboayue/rust-ibapi/compare/v3.2.1...v3.3.0
[3.2.1]: https://github.com/wboayue/rust-ibapi/compare/v3.2.0...v3.2.1
[3.2.0]: https://github.com/wboayue/rust-ibapi/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/wboayue/rust-ibapi/compare/v3.0.1...v3.1.0
[3.0.1]: https://github.com/wboayue/rust-ibapi/releases/tag/v3.0.1
