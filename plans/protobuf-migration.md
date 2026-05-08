# Protobuf Migration Tracker

On `main` (v3.0) the text protocol is **gone**. Every outgoing request is encoded as a protobuf message via `crate::messages::encode_protobuf_message(msg_id, &request.encode_to_vec())` and `prost` is a required dependency (not feature-gated). See PRs #449–#452 for the phased migration that landed.

This tracker therefore is no longer about flipping individual messages from text to proto — it's about **gaps**: outgoing messages in the IBKR protobuf set that we don't yet expose as a public Rust API.

## Source of truth

Canonical mapping of which `OutgoingMessages` is protobuf at which server version:
`/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs` (`PROTOBUF_MSG_IDS` dictionary, 79 entries).

Server version constants in `src/server_versions.rs`: `PROTOBUF`=201 through `PROTOBUF_REST_MESSAGES_3`=213.

## Status legend

- [x] migrated and exposed via `Client` API (sync + async)
- [~] encoder/decoder exists, public API still missing
- [ ] not implemented in rust-ibapi at all
- [n/a] intentionally skipped (handshake-only, deprecated, or not part of public surface)

## Migrated APIs (encoder + decoder + Client method)

### Orders & executions
- [x] `PlaceOrder` — `Client::place_order` / `submit_order`
- [x] `CancelOrder` — `Client::cancel_order`
- [x] `RequestGlobalCancel` — `Client::global_cancel`
- [x] `RequestExecutions` — `Client::executions`
- [x] `RequestOpenOrders` — `Client::open_orders`
- [x] `RequestAllOpenOrders` — `Client::all_open_orders`
- [x] `RequestAutoOpenOrders` — `Client::auto_open_orders`
- [x] `RequestCompletedOrders` — `Client::completed_orders`
- [x] `RequestIds` — `Client::next_order_id` (next valid id)
- [x] `ExerciseOptions` — `Client::exercise_options`

### Contracts
- [x] `RequestContractData` — `Client::contract_details`
- [x] `RequestMatchingSymbols` — `Client::matching_symbols`
- [x] `RequestMarketRule` — `Client::market_rule`
- [x] `RequestSecurityDefinitionOptionalParameters` — `Client::option_chain`
- [x] `ReqCalcOptionPrice` — `Client::calculate_option_price`
- [x] `ReqCalcImpliedVolat` — `Client::calculate_implied_volatility`
- [x] `CancelOptionPrice` — internal cancel (`encode_cancel_option_computation`)
- [x] `CancelImpliedVolatility` — internal cancel (`encode_cancel_option_computation`)
- [x] `CancelContractData` — internal cancel

### Market data — realtime
- [x] `RequestMarketData` — `Client::market_data`
- [x] `CancelMarketData` — subscription drop
- [x] `RequestMarketDepth` — `Client::market_depth`
- [x] `CancelMarketDepth` — subscription drop
- [x] `RequestMktDepthExchanges` — `Client::market_depth_exchanges`
- [x] `RequestMarketDataType` — `Client::switch_market_data_type`
- [x] `RequestRealTimeBars` — `Client::realtime_bars`
- [x] `CancelRealTimeBars` — subscription drop
- [x] `RequestTickByTickData` — `Client::tick_by_tick_*`
- [x] `CancelTickByTickData` — subscription drop

### Market data — historical
- [x] `RequestHistoricalData` — `Client::historical_data`
- [x] `CancelHistoricalData` — subscription drop
- [x] `RequestHeadTimestamp` — `Client::head_timestamp`
- [x] `CancelHeadTimestamp` — internal cancel
- [x] `RequestHistogramData` — `Client::histogram_data`
- [x] `CancelHistogramData` — subscription drop
- [x] `RequestHistoricalTicks` — `Client::historical_ticks_*`
- [x] `CancelHistoricalTicks` — internal cancel

### Accounts & positions
- [x] `RequestAccountData` — `Client::account_updates`
- [x] `RequestManagedAccounts` — `Client::managed_accounts`
- [x] `RequestPositions` — `Client::positions`
- [x] `CancelPositions` — subscription drop
- [x] `RequestAccountSummary` — `Client::account_summary`
- [x] `CancelAccountSummary` — subscription drop
- [x] `RequestPositionsMulti` — `Client::positions_multi`
- [x] `CancelPositionsMulti` — subscription drop
- [x] `RequestAccountUpdatesMulti` — `Client::account_updates_multi`
- [x] `CancelAccountUpdatesMulti` — subscription drop
- [x] `RequestPnL` — `Client::pnl`
- [x] `CancelPnL` — subscription drop
- [x] `RequestPnLSingle` — `Client::pnl_single`
- [x] `CancelPnLSingle` — subscription drop
- [x] `RequestFamilyCodes` — `Client::family_codes`
- [x] `RequestCurrentTime` — `Client::server_time`
- [x] `RequestCurrentTimeInMillis` — `Client::server_time_millis`

### News
- [x] `RequestNewsProviders` — `Client::news_providers`
- [x] `RequestNewsBulletins` — `Client::news_bulletins`
- [x] `CancelNewsBulletin` — subscription drop
- [x] `RequestNewsArticle` — `Client::news_article`
- [x] `RequestHistoricalNews` — `Client::historical_news`

### Scanner
- [x] `RequestScannerParameters` — `Client::scanner_parameters`
- [x] `RequestScannerSubscription` — `Client::scanner_subscription`
- [x] `CancelScannerSubscription` — subscription drop

### Display groups
- [x] `QueryDisplayGroups` — `Client::query_display_groups`
- [x] `SubscribeToGroupEvents` — `Client::subscribe_to_group_events`
- [x] `UpdateDisplayGroup` — `Client::update_display_group`
- [x] `UnsubscribeFromGroupEvents` — subscription drop

### WSH (Wall Street Horizon)
- [x] `ReqWshMetaData` — `Client::wsh_metadata`
- [x] `CancelWshMetaData` — subscription drop
- [x] `ReqWshEventData` — `Client::wsh_event_data_*`
- [x] `CancelWshEventData` — subscription drop

### Connection
- [x] `StartApi` — encoded by `connection/common.rs`

## Outgoing messages with no public Rust API yet

These appear in `PROTOBUF_MSG_IDS` but are not exposed by `Client`. They need an encoder + decoder (where applicable) + `pub fn` on `Client` for both sync and async.

- [ ] `RequestFundamentalData` / `CancelFundamentalData` (SCAN_DATA, server 210)
- [ ] `RequestSmartComponents` (REST_MESSAGES_2, server 212)
- [ ] `RequestSoftDollarTiers` (REST_MESSAGES_2, server 212)
- [ ] `ReqUserInfo` (REST_MESSAGES_2, server 212)
- [ ] `RequestFA` / `ReplaceFA` (REST_MESSAGES_1, server 211) — Financial Advisor account/group config
- [ ] `ChangeServerLog` (REST_MESSAGES_3, server 213) — server-side log level
- [ ] `VerifyRequest` / `VerifyMessage` (REST_MESSAGES_3, server 213) — extension auth handshake

## Intentionally skipped

- [n/a] `VerifyAndAuthRequest` / `VerifyAndAuthMessage` — not in `PROTOBUF_MSG_IDS`; legacy text-only and unused in our client
- [n/a] Handshake bytes before `StartApi` (version negotiation) — protocol-level, not message-level

## Per-API "done" checklist (for the gaps above)

1. Encoder in the appropriate domain's `common/encoders.rs` building a `proto::*Request` via `prost::Message::encode_to_vec()` and `encode_protobuf_message`
2. Encoder unit test asserting the framed `msg_id` and a round-trip decode
3. Decoder in `common/decoders/` (if the response type isn't already wired)
4. Public `pub fn` on `Client` in `<domain>/sync/mod.rs` and `pub async fn` in `<domain>/async/mod.rs`
5. Default + `--features sync` + `--all-features` builds compile, clippy clean
6. Integration test against a live gateway using `place_order`-style drain pattern when a message is fire-and-forget on the wire (see `docs/integration-tests.md` §4)
