# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [4.0.0] - 2026-09-03

### Added

- The prelude now re-exports the account-parameter newtypes `AccountGroup`, `AccountId`, `ContractId`, and `ModelCode` (from `ibapi::accounts::types`). These are required arguments of `account_summary`, `account_updates`, `pnl`, `pnl_single`, and `positions_multi`, but were the only such parameter types missing from `ibapi::prelude` — the quick-start example could not be written with prelude-only imports.

- `SUBSCRIPTION_LAG_CODE` (`-6`), a synthesized notice code delivered **in-band** on an async subscription whose consumer fell behind its broadcast channel: the channel evicts the oldest frames, and the subscription now receives a non-terminal `SubscriptionItem::Notice` naming the dropped count (plus a `warn!`) where it previously resumed silently — the frames were simply gone with no signal at any level. Reconcile as after a reconnect gap (order streams: `open_orders()`; market data self-corrects with the next tick). Step 1 of the #779 plan (`plans/broadcast-lag-visibility.md`): loss is now observable; the bounded-lossy semantic itself is unchanged (#779).

- `ClientBuilder::channel_capacity` (async only), setting the per-subscription broadcast channel capacity (default unchanged: 1024). Raise it if consumers legitimately fall behind during bursts; `0` is rejected as `Error::InvalidArgument`. The notice fan-out keeps the default (#779).

- `ClientBuilder::max_reconnect_attempts` and `ClientBuilder::reconnect_forever` (sync and async), configuring how many times the automatic reconnect loop retries before giving up. The default is unchanged: 20 attempts, ~7.5 minutes with the capped Fibonacci backoff — too short to span a TWS/IB Gateway nightly restart that needs a manual re-login, which is what `reconnect_forever` (retry every 30 s until the connection returns) is for (#762, #763).

- `IBAPI_RAW_CAPTURE_DIR`, a byte-level tap on the inbound stream. Set it to a directory and every connection writes `<stamp>-<n>-inbound-<NNN>.bin`, a verbatim copy of what the socket delivered — **including the 4-byte length prefix** — alongside an `.idx` of `seq,utc_timestamp,offset,declared_length` per frame. `IBAPI_RECORDING_DIR` cannot substitute: it is handed an already-parsed message and re-frames it, so the prefix it writes is one this crate computed. That makes it blind to a framing desync, where the prefix is the corrupted field. The tap sits below validation, so a prefix rejected as `Error::InvalidFrame` still reaches the capture — that prefix is the evidence. A reconnect starts a new file, so no `.bin` splices two TCP streams. Because prefixes are preserved, a capture replays through the frame reader unchanged; `cargo run --example replay_raw_capture -- <file>.bin` walks one and reports the first frame that cannot be read. Captures are unredacted wire bytes, so they carry account ids, positions, and orders.

- `UNKNOWN_MESSAGE_TYPE_CODE` (`-5`), a synthesized notice code published to `Client::notice_stream` when a frame's message id maps to no known `IncomingMessages` kind. Joins the existing client-side sentinels (`HANDSHAKE_UNKNOWN_FRAME_CODE`, `HANDSHAKE_DECODE_FAILURE_CODE`); TWS itself only uses codes 0 and up. This is the observable form of a framing desynchronization. The notice text names the offending id, which is what separates the two explanations: a slipped stream yields scattered ids, while a message type IBKR has added repeats one.

- `Error::InvalidFrame`, returned when a frame's 4-byte length prefix cannot describe a TWS message — shorter than the message id, or larger than the 16 MiB cap the official client enforces as `Constants.MaxMsgSize`. Classified as `Error::is_connection_lost`, so both dispatchers reconnect: the framing is positional, with no delimiter to re-anchor on, and a reconnect is the only recovery. `Error` is `#[non_exhaustive]`, so the new variant is not a breaking change.

- `Error::UnexpectedWireFormat`, returned when a message arrives in the wrong wire format for the reader handling it — text framing at a proto-only decoder, or proto framing at a text-field accessor. Previously this shared `Error::UnexpectedResponse` with the unrelated "message is not for this decoder" case, which the dispatcher skips silently. `Error` is `#[non_exhaustive]`, so the new variant is not a breaking change (#731).

- `Notice.request_id`, the originating request or order id (`None` for request-less notices). `Notice` is not `#[non_exhaustive]`, so struct literals must add the field (#759).

- `ORDER_MESSAGE_CODE` (`399`), the generic order-message code whose message text decides severity (#759).

### Changed

- `Client::option_chain` returns a builder instead of taking `exchange` positionally: `client.option_chain(symbol, security_type, contract_id)` with an optional `.exchange(..)` setter and a `.subscribe()` terminal. The old signature's `exchange: &str` documented `""` as "all exchanges", and live checks against TWS show that is the only value that returns anything for a stock underlying — the field is TWS's `futFopExchange`, a futures-options filter, and `"SMART"` (which the async example passed) yields an empty chain. `contract_id` stays required: `0` (which two examples passed) is rejected with code 321 "Invalid contract id". An unset exchange is now omitted from the wire rather than sent as an empty string, matching the official client. See `docs/migration-4.0.md` §10.

- Sync transport: a stalled consumer now triggers `warn!` watermarks at every 10,000 queued messages on the subscription, shared-channel, and order-update send paths. Sync channels are unbounded and never drop; their failure mode — silent memory growth — is now loud. The notice fan-out (`NoticeBroadcaster`) is excluded for now; see `plans/broadcast-lag-visibility.md` (#779).

- `OrderStatusKind` gains an `Unknown(String)` variant preserving unrecognized status strings (see the Fixed entry below). Breaking: the enum stays deliberately exhaustive like `Liquidity::Unknown` (#760), so downstream exhaustive matches need a new arm; `Copy` is gone (still `Clone`); `as_str()` returns `&str`; `is_active()` / `is_terminal()` take `&self` and are `false` for `Unknown`. Serialization stays a plain string in both directions, and the `utoipa` schema is now a plain `string` to match. Details and examples in `docs/migration-4.0.md` §9 (#774).

- `MarketDataBuilder` moved from `market_data::builder` to `market_data::realtime`, alongside the sibling builders (`RealtimeBarsBuilder`, `MarketDepthBuilder`, `TickByTickBuilder`) it always belonged with; the `market_data::builder` module is gone. `MarketDataBuilder::new` is also `pub(crate)` now, matching its siblings — construct via `client.market_data(&contract)`. Only code naming the type — imports, signatures, or a direct `new` call — breaks; `client.market_data(&contract)` call sites are unaffected. The prelude now exports all four realtime builders (`MarketDataBuilder` and `RealtimeBarsBuilder` join the two already there). See `docs/migration-4.0.md` §7 (#772).

- A text-framed message reaching a proto-only decoder now fails the subscription instead of being skipped. At `server_versions::PROTOBUF_REST_MESSAGES_3` every message with a proto decoder arrives proto-framed, so reaching this means the gateway broke protocol — previously the message was dropped and the subscription silently yielded nothing. Wrong-message-type frames are still skipped, which is what shared channels need (#731).
- Historical tick and histogram sizes are now `Option<f64>` instead of `i32`: `TickMidpoint.size`, `TickLast.size`, `TickBidAsk.size_bid`/`size_ask`, and `HistogramEntry.size`. IBKR models these as decimals on the wire, and the old `i32` parse silently truncated fractional sizes — a crypto tick of `0.5` decoded as `0`. `None` means TWS sent no value (field absent, empty, or an "unset" sentinel); `Some(0.0)` is a real zero. This also changes the serialized shape — a size is now `100.0` rather than `100`, absent is `null` rather than `0`, and the `utoipa` schema becomes a nullable `number` (#716).
- `ContractDetails.min_size`, `size_increment`, and `suggested_size_increment` are now `Option<f64>` instead of `f64`. Contracts without size rules omit these on the wire, where the old `0.0` was indistinguishable from a real value and a `size_increment` of `0.0` is nonsense (#716).
- Every one-shot request now narrows the inbound frame to the message type it asked for before decoding it. Narrowing used to be opt-in — each domain hand-wrote a `decode_*_message` wrapper for it, and 26 of the 50 sites had no wrapper, decoding whatever arrived on the shared channel. A foreign frame now surfaces as `Error::UnexpectedResponse` naming both the expected and received type, rather than being fed to the wrong payload decoder — where overlapping protobuf field numbers usually produce a plausible struct full of wrong values instead of an error. Affects `next_valid_order_id` in particular, which reads the shared `RequestIds` channel (#738).
- `market_rule`, `family_codes`, `calculate_option_price`, `calculate_implied_volatility`, and `next_valid_order_id` now retry a connection reset up to three times, like every other one-shot request. They previously surfaced `Error::ConnectionReset` to the caller on the first reset — a gateway reconnect mid-request failed the call outright. Nothing about these five made them unsafe to retry; they were the sites that had picked a helper without retry, `market_rule` and `family_codes` because the only helper bundling a server-version check was the non-retrying one (#741).
- `head_timestamp`, `histogram_data`, `market_depth_exchanges`, and `historical_schedules(..).fetch()` retry a connection reset at most three times instead of unboundedly. The async `head_timestamp` recursed on a closed stream and the async `histogram_data`, `market_depth_exchanges`, and schedule fetch looped on one; a gateway that keeps resetting would hang the call rather than return. They also now agree with their blocking twins on what a closed stream means: `Error::UnexpectedEndOfStream` for `head_timestamp` and the schedule fetch, an empty list for `histogram_data` and `market_depth_exchanges` (#738).

- `Liquidity` now preserves unrecognized execution liquidity codes as a new `Unknown(i32)` variant instead of collapsing them to `Liquidity::None`. The documented codes 0–3 decode as before; any other value surfaces as `Unknown(code)` so callers can log, store, or reject it — previously an unknown code was indistinguishable from a genuine "no liquidity information", which is silent data loss (the official C# client has the same coercion). No official client or `Execution.proto` defines a code outside 0–3 today, so this is forward compatibility. `Liquidity` is deliberately exhaustive (no `#[non_exhaustive]`), so downstream exhaustive matches need a new arm — breaking (#760).

- `Client::wsh_event_data_by_contract` and `Client::wsh_event_data_by_filter` return builders instead of taking optional arguments positionally. The first took one required argument and four `Option`s, the second one and two; every call site in this repository passed `None` for all of them. Narrowing is now `.starting(date)` / `.ending(date)` / `.limit(n)` / `.auto_fill(spec)`, with `.fetch()` (single result) and `.subscribe()` (subscription) as the terminals. See `docs/migration-4.0.md` §3 (#752).

- Code 399 order messages whose text carries a `Warning:` line now classify as warnings (`Notice::is_warning`, `NoticeCategory::Warning`) and route as non-terminal notices. TWS uses 399 for both severities — e.g. "order will not be placed at the exchange until …" for outside-RTH stops — and the warning form previously terminated the owning subscription as an order rejection (#759).

- The order-update stream delivers order-bound error frames as `SubscriptionItem::Notice` (carrying `request_id`, code, and message) instead of raw error frames that `OrderUpdate` could not decode — consumers previously saw `Err(Error::UnexpectedResponse)`. Note `filter_data()` / `iter_data()` drop notices; match on `SubscriptionItem::Notice` to observe rejections of fire-and-forget orders. Request-less errors and errors owned by a data-request subscription no longer reach the stream at all (#759).

### Removed

- The free function `market_data::realtime::sync::market_data()` is crate-private, matching the `realtime_bars` / `market_depth` / `tick_by_tick` free functions beside it (in sync-only builds it was also reachable as `market_data::realtime::market_data` through the glob re-export). It was the low-level request under the `client.market_data(&contract)` builder, which is the supported path and takes the same inputs via setters; no call site in `examples/` or the integration crates used the free function. See `docs/migration-4.0.md` §8 (#780).

- `Client::check_server_version()` is crate-private. It was `pub` on the async client and `pub(crate)` on the blocking one — drift rather than design, since it is the internal guard every version-gated API already calls. Compare against `Client::server_version()` directly if you need to branch on gateway support (#728).

### Fixed

- `ComboLegOpenClose::from(i32)` no longer panics on a value outside `0..=3`; unrecognised values decode to `ComboLegOpenClose::Unknown`. The `From<i32>` impl runs inside contract-details decoding, so an unexpected `openClose` from TWS took the dispatcher down instead of surfacing as an `Unknown` leg.

- `HistoricalDataEnd` and historical-schedule decoding no longer panics when a wall-clock time from TWS falls in a DST fold or gap, and the connection time reported at handshake is no longer dropped for a fold. A reading in a repeated hour resolves to its earlier occurrence (the pre-transition offset). A reading in a skipped hour never showed on a clock, so it can only come from TWS doing date arithmetic on wall-clock time — a window start computed as "end minus 300 days", say — and is pushed forward by the gap: 02:30 on a US spring-forward day becomes 03:30 EDT. Previously a 300-day `historical_data()` request whose start edge landed on a fall-back night panicked with `OffsetResult::unwrap` (#790).

- An `OrderStatus` or `OpenOrder` frame carrying an unrecognized status string no longer terminates the subscription delivering it. Decoding failed with `Error::Parse`, which ended `place_order`, `cancel_order`, `open_orders`, and — worst — the long-lived `order_update_stream` the moment TWS shipped a status this crate had not modeled; the official IB client parses such statuses into an `Unknown` fallback instead of failing. The status now arrives as `OrderStatusKind::Unknown(raw)` and the stream continues (#774).

- Async `open_orders()`, `all_open_orders()`, `completed_orders()`, and the other streaming shared-channel subscriptions (positions, account data, news bulletins) no longer hang forever when a reconnect happens mid-request. After a reconnect, `reset_channels` notified request-id and order-id subscriptions with `Error::ConnectionReset` but left shared channels untouched — and the end marker such a subscription awaits is a response to a request sent on the dead connection, so nothing ever arrived. The async reset now notifies shared-channel senders too, mirroring the sync transport (which already did this). Subscriptions created after the reset are unaffected: they subscribe at the channel's current tail and never see the injected error (#776).

- Dropping a subscription no longer unregisters a newer subscription under the same key. Drop-signal cleanup crosses a channel to a separate task/thread, so it can run arbitrarily late — after `place_order` then `cancel_order` re-registered the same order id, or after the order-update stream was recreated — and removal was unconditional, silently disconnecting the live replacement (data and error frames for that order went nowhere). The async cleanup now removes a registration only when its channel has no receivers left (a dropping subscription detaches its receivers first, so the count is authoritative); the sync drop signal now carries the dropped subscription's sender and cleanup removes only the matching registration. Dropping a clone while a sibling is still consuming no longer unregisters the shared channel either (#773).

- `order_update_stream()` can be dropped and immediately recreated. Recreation returned `Error::AlreadySubscribed` until the asynchronous cleanup ran, and a stale cleanup signal could then clear the replacement's registration, after which the new stream silently received nothing. The async transport now replaces a registration whose channel has no receivers instead of refusing, and stale signals cannot clear a live replacement on either transport. Sync note: recreation still returns `AlreadySubscribed` for the instant between drop and the cleanup thread running (crossbeam senders expose no receiver count); retry on `AlreadySubscribed` there (#778).

- The order cancellation confirmation (code 202) is routed to request-bound subscriptions as a non-terminal `SubscriptionItem::Notice` instead of a stream-terminating `Err`. Successfully cancelling an order ended the `cancel_order` (or `place_order`) subscription with an error whose payload the crate itself classifies as informational (`Notice::is_cancellation()`, `NoticeCategory::Cancellation`), while the order-update stream received the identical frame as a non-terminal notice. Routing now agrees with that classification. Behavior note: the 202-`Err` was also the only thing that ended a `cancel_order` subscription; it now stays open until dropped — consume it like the order-update stream and break on `notice.is_cancellation()` (#775).

- `FibonacciBackoff` no longer overflows `u64` on the 93rd consecutive `next_delay()` call. The backoff state kept growing past `max` (only the returned delay was capped), so a reconnect loop with a large `max_reconnect_attempts` or `reconnect_forever` would panic with `attempt to add with overflow` in debug builds, or wrap to nonsense delays in release, once an outage outlasted ~92 attempts. The state is now clamped at `max`. Returned delays are unchanged.

- Notice 2188 ("Up-to-the-second historical data requires additional subscription for the API.") is classified as a data advisory instead of a hard error. TWS sends it and then delivers the historical bars anyway — the account merely lacks the up-to-the-second tail — but `historical_data()` returned `Err` on the notice and discarded the bars that followed, so accounts without the real-time entitlement for a listing got no history at all for those symbols. `DATA_ADVISORY_CODES` widens from `[i32; 2]` to `[i32; 3]`, which is breaking only for code binding the const with an explicit array type (#765).

- Error 10090 ("Part of requested market data is not subscribed. Subscription-independent ticks are still active") is classified as a data advisory like 10089 and 10167: it is published as a non-terminal notice instead of ending the market-data subscription. TWS sends it on partial entitlement — commonly an options subscription without the underlying — and keeps delivering the ticks the account is entitled to, but the subscription was torn down before they could arrive. `DATA_ADVISORY_CODES` widens again to `[i32; 4]` (#768).

- A notice whose protobuf `error_code` field is absent no longer fails every in-flight one-shot shared request (`server_time`, `managed_accounts`, `next_valid_order_id`, ...). The absent field decodes to code 0 — outside every warning range — so IB Gateway's code-less informational notices (e.g. "Warning: Approaching max rate of 50 messages per second (42)") were treated as request-less hard errors. Code 0 now classifies as a warning throughout: request-less frames reach `Client::notice_stream` without failing anything, frames carrying a request id are delivered to that subscription as a notice instead of terminating it, and `Notice::is_warning()` / `Notice::category()` agree with the routing. An error frame that fails proto decode falls back to the same code-0 path and now logs a warning naming the byte length (#766).

- The frame reader now validates the 4-byte length prefix before using it, instead of trusting it to size an allocation and a `read_exact`. Nothing bounded it: four garbage bytes were read as a body length of up to 4 GiB, which allocated that much and then blocked until that many bytes arrived — consuming and destroying every real message in between, then yielding one bogus frame with the stream left permanently mis-framed. Because the framing is positional, nothing re-synchronizes it, and a mis-framed protobuf payload still decodes without error (prost skips unrecognized field numbers), so the visible symptom was plausible-looking wrong field values that never recovered. Out-of-range prefixes now raise `Error::InvalidFrame` and drive a reconnect. The cap matches the official client's `Constants.MaxMsgSize` (`EReader.readSingleMessage`, which raises `BAD_LENGTH`).

- A frame that no channel claims is now reported instead of dropped. An unrecognized message id raises a `UNKNOWN_MESSAGE_TYPE_CODE` notice and a warning; a known type with no current subscriber stays at `info`, as before, since that is ordinary steady state. Previously the blocking client logged every such frame at `info` without distinguishing the two, and the async client logged nothing at all — so a desynchronized stream was indistinguishable from an idle one, which is why the incident that prompted this work surfaced data-farm notices and no decode error.

- A frame whose body is too short to hold the 4-byte message id no longer panics the dispatcher. `parse_raw_message` indexed the first four bytes unguarded, so a body of 0–3 bytes aborted with `index out of bounds` — killing the dispatcher thread on the blocking client and the dispatcher task on the async one. It returns `Error::InvalidFrame` now.

- `IBAPI_RECORDING_DIR` pointing at an unwritable path no longer panics during `Client::connect`. Creating the recording directory was `unwrap`ed, so a typo or a read-only mount aborted the process rather than the recording. It now warns and disables recording, matching `IBAPI_RAW_CAPTURE_DIR` — a diagnostic aid must not be the reason a connection fails.

- `IBAPI_RECORDING_DIR` records an unrecognized message id as itself rather than as `-1`. The recorder reconstructed the frame's id from the resolved `IncomingMessages` kind, and every unrecognized id resolves to `NotValid`, whose discriminant is `-1` — so the capture of a framing desync, which is exactly the capture worth replaying, carried a fabricated id where the offending one should have been. Recognized ids are unaffected: `IncomingMessages::from` maps a value to the variant with that discriminant, so the two agreed for every frame the client understands.

- `IBAPI_RECORDING_DIR` records the actual response payload again. Recording wrote a response's *parsed text fields*, and a protobuf frame has none — so since the protobuf-only transition every `NNNN-response.msg` held the bare message id and nothing else. Responses are now written as their wire frame (4-byte big-endian message id followed by the payload), matching what `record_request` already wrote for outbound messages. Text-framed responses keep their pipe-delimited form (#748).

- `historical_data(..).fetch()` on the async client now retries a connection reset instead of surfacing it, and no longer retries a closed stream. It had the two cases backwards relative to its blocking twin: a routed `Error::ConnectionReset` returned to the caller on the first occurrence, while an empty stream was re-sent five times before failing as `Error::ConnectionReset`. Both sides now share `retry_on_connection_reset` — up to three retries on a reset, `Error::UnexpectedEndOfStream` on a closed stream, on the first attempt. An error on the follow-on `HistoricalDataEnd` frame is also propagated rather than dropped, on both sides (#744).

- `OrderBuilder::analyze()` (what-if orders, blocking and async) now returns the TWS rejection instead of `Error::UnexpectedEndOfStream`. A rejected what-if order arrives as a routed error, which the response read discarded — the blocking path via `if let Ok(..)` inside the loop, the async path by ending its `while let Some(Ok(..))` loop — so the caller lost the reason (e.g. code 201, `Order rejected - reason:...`) and got a generic end-of-stream error. Rejection is a routine outcome for a what-if order, so this was the likeliest path to hit it (#735).

- Reconnection now survives the window where TWS accepts the TCP connection but its API handshake is not yet ready — common during an automated restart. A session-establishment failure (handshake, `startAPI`, account info) used to abort the reconnect loop on the first occurrence; it now consumes one attempt and follows the same fibonacci backoff as a socket failure, on both the blocking and async clients. When every attempt fails, `reconnect` returns the last attempt's error instead of a bare `Error::ConnectionFailed`, so a permanent cause — say, an incompatible server version — is named rather than hidden behind a generic failure (#761).

- `matching_symbols()` on the blocking client now returns the TWS error instead of an empty list. A routed error arrives as `Some(Err(_))`, which the `if let Some(Ok(_))` read discarded, so a rejected pattern silently returned `Ok(vec![])` — indistinguishable from "no symbols matched". The async client already propagated it (#735).

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

[Unreleased]: https://github.com/wboayue/rust-ibapi/compare/v4.0.0...HEAD
[4.0.0]: https://github.com/wboayue/rust-ibapi/compare/v3.3.0...v4.0.0
[3.3.0]: https://github.com/wboayue/rust-ibapi/compare/v3.2.1...v3.3.0
[3.2.1]: https://github.com/wboayue/rust-ibapi/compare/v3.2.0...v3.2.1
[3.2.0]: https://github.com/wboayue/rust-ibapi/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/wboayue/rust-ibapi/compare/v3.0.1...v3.1.0
[3.0.1]: https://github.com/wboayue/rust-ibapi/releases/tag/v3.0.1
