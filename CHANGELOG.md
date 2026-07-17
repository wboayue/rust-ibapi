# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
