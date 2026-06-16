# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/wboayue/rust-ibapi/compare/v3.0.1...HEAD
[3.0.1]: https://github.com/wboayue/rust-ibapi/releases/tag/v3.0.1
