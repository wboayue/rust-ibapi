# Phase 4: Outbound Protobuf Encoding — Replace Text Encoders

## Context

All outbound requests currently build a `RequestMessage` with NUL-delimited text fields via `push_field()`. Replace with protobuf serialization: build proto request objects, serialize with prost, and send as binary payloads with +200 message ID offset.

**Depends on:** Phase 2 (protobuf message writing support in transport layer)

## Architecture

```
domain types → proto types → prost::Message::encode() → binary payload
                  ↑
     src/proto/encoders.rs (shared converters)
```

**Shared converters** in `src/proto/encoders.rs` handle domain→proto for reused types:
- `contracts::Contract` → `proto::Contract`
- `orders::Order` → `proto::Order`
- `contracts::ComboLeg` → `proto::ComboLeg`
- `contracts::DeltaNeutralContract` → `proto::DeltaNeutralContract`
- `orders::SoftDollarTier` → `proto::SoftDollarTier`

**Domain encoders** in each module's `encoders.rs` build the top-level proto request and serialize.

## Encoding Pattern

```rust
// src/proto/encoders.rs
pub fn encode_contract(contract: &Contract) -> proto::Contract {
    proto::Contract {
        con_id: Some(contract.contract_id),
        symbol: Some(contract.symbol.to_string()),
        sec_type: Some(contract.security_type.to_string()),
        // ... field-by-field mapping
        ..Default::default()
    }
}

// src/orders/common/encoders.rs
pub(in crate::orders) fn encode_place_order(
    server_version: i32,
    order_id: i32,
    contract: &Contract,
    order: &Order,
) -> Result<Vec<u8>, Error> {
    let proto_req = proto::PlaceOrderRequest {
        order_id: Some(order_id),
        contract: Some(proto::encoders::encode_contract(contract)),
        order: Some(proto::encoders::encode_order(order)),
        ..Default::default()
    };
    let mut buf = Vec::new();
    proto_req.encode(&mut buf)?;
    Ok(encode_protobuf_message(OutgoingMessages::PlaceOrder as i32, &buf))
}
```

C# reference: `EClientUtils.cs` — 72 `create*Proto()` methods.

## Domain Modules to Update

### Orders (`src/orders/common/encoders.rs`)
- `encode_place_order()` → `proto::PlaceOrderRequest`
- `encode_cancel_order()` → `proto::CancelOrderRequest`
- `encode_request_open_orders()` → `proto::OpenOrdersRequest`
- `encode_request_all_open_orders()` → `proto::AllOpenOrdersRequest`
- `encode_request_auto_open_orders()` → `proto::AutoOpenOrdersRequest`
- `encode_request_completed_orders()` → `proto::CompletedOrdersRequest`
- `encode_request_executions()` → `proto::ExecutionRequest`
- `encode_global_cancel()` → `proto::GlobalCancelRequest`

### Contracts (`src/contracts/common/encoders.rs`)
- `encode_request_contract_data()` → `proto::ContractDataRequest`
- `encode_cancel_contract_data()` → `proto::CancelContractData`

### Market Data - Realtime (`src/market_data/realtime/common/encoders.rs`)
- `encode_request_market_data()` → `proto::MarketDataRequest`
- `encode_cancel_market_data()` → `proto::CancelMarketData`
- `encode_request_market_depth()` → `proto::MarketDepthRequest`
- `encode_cancel_market_depth()` → `proto::CancelMarketDepth`
- `encode_request_tick_by_tick()` → `proto::TickByTickRequest`
- `encode_cancel_tick_by_tick()` → `proto::CancelTickByTick`

### Market Data - Historical (`src/market_data/historical/common/encoders.rs`)
- `encode_request_historical_data()` → `proto::HistoricalDataRequest`
- `encode_cancel_historical_data()` → `proto::CancelHistoricalData`
- `encode_request_realtime_bars()` → `proto::RealTimeBarsRequest`
- `encode_cancel_realtime_bars()` → `proto::CancelRealTimeBars`
- `encode_request_historical_ticks()` → `proto::HistoricalTicksRequest`
- `encode_cancel_historical_ticks()` → `proto::CancelHistoricalTicks`
- `encode_request_head_timestamp()` → `proto::HeadTimestampRequest`
- `encode_cancel_head_timestamp()` → `proto::CancelHeadTimestamp`
- `encode_request_histogram_data()` → `proto::HistogramDataRequest`
- `encode_cancel_histogram_data()` → `proto::CancelHistogramData`

### Accounts (`src/accounts/common/encoders.rs`)
- `encode_request_account_data()` → `proto::AccountDataRequest`
- `encode_request_positions()` → `proto::PositionsRequest`
- `encode_cancel_positions()` → `proto::CancelPositions`
- `encode_request_account_summary()` → `proto::AccountSummaryRequest`
- `encode_cancel_account_summary()` → `proto::CancelAccountSummary`
- `encode_request_pnl()` → `proto::PnLRequest`
- `encode_cancel_pnl()` → `proto::CancelPnL`
- `encode_request_pnl_single()` → `proto::PnLSingleRequest`
- `encode_cancel_pnl_single()` → `proto::CancelPnLSingle`
- `encode_request_positions_multi()` → `proto::PositionsMultiRequest`
- `encode_cancel_positions_multi()` → `proto::CancelPositionsMulti`
- `encode_request_account_updates_multi()` → `proto::AccountUpdatesMultiRequest`
- `encode_cancel_account_updates_multi()` → `proto::CancelAccountUpdatesMulti`

### News (`src/news/common/encoders.rs`)
- `encode_request_news_bulletins()` → `proto::NewsBulletinsRequest`
- `encode_cancel_news_bulletins()` → `proto::CancelNewsBulletins`
- `encode_request_news_article()` → `proto::NewsArticleRequest` (if exists)
- `encode_request_news_providers()` → `proto::NewsProvidersRequest`
- `encode_request_historical_news()` → `proto::HistoricalNewsRequest`

### Scanner (`src/scanner/common/encoders.rs`)
- `encode_request_scanner_parameters()` → `proto::ScannerParametersRequest`
- `encode_request_scanner_subscription()` → `proto::ScannerSubscriptionRequest`
- `encode_cancel_scanner_subscription()` → `proto::CancelScannerSubscription`

### Display Groups (`src/display_groups/common/encoders.rs`)
- `encode_query_display_groups()` → `proto::QueryDisplayGroupsRequest`
- `encode_subscribe_to_group_events()` → `proto::SubscribeToGroupEventsRequest`
- `encode_update_display_group()` → `proto::UpdateDisplayGroupRequest`
- `encode_unsubscribe_from_group_events()` → `proto::UnsubscribeFromGroupEventsRequest`

### WSH (`src/wsh/common/encoders.rs`)
- `encode_request_wsh_metadata()` → `proto::WshMetaDataRequest`
- `encode_cancel_wsh_metadata()` → `proto::CancelWshMetaData`
- `encode_request_wsh_event_data()` → `proto::WshEventDataRequest`
- `encode_cancel_wsh_event_data()` → `proto::CancelWshEventData`

### Market Data module-level (`src/market_data/mod.rs`)
- `encode_request_market_data_type()` → `proto::MarketDataTypeRequest`

## Files Modified

| File | Change |
|------|--------|
| `src/proto/mod.rs` | Add `pub mod encoders;` |
| `src/proto/encoders.rs` | (new) Shared domain→proto converters |
| `src/orders/common/encoders.rs` | Rewrite to protobuf |
| `src/contracts/common/encoders.rs` | Rewrite to protobuf |
| `src/market_data/realtime/common/encoders.rs` | Rewrite to protobuf |
| `src/market_data/historical/common/encoders.rs` | Rewrite to protobuf |
| `src/market_data/mod.rs` | Rewrite market data type encoder |
| `src/accounts/common/encoders.rs` | Rewrite to protobuf |
| `src/news/common/encoders.rs` | Rewrite to protobuf |
| `src/scanner/common/encoders.rs` | Rewrite to protobuf |
| `src/display_groups/common/encoders.rs` | Rewrite to protobuf |
| `src/wsh/common/encoders.rs` | Rewrite to protobuf |

## C# Reference

| File | What |
|------|------|
| `EClientUtils.cs` lines 12-1248 | All 72 `create*Proto()` methods — primary reference |
| `EClient.cs` lines 193-213 | `startApiProtoBuf()` — send pattern |
| `IBParamsList.cs` lines 16-35 | Binary message ID + proto bytes encoding |
| `Constants.cs` lines 22-103 | `PROTOBUF_MSG_IDS` — which messages use protobuf at which version |

## Verification

```bash
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
```

Unit tests: encode domain objects → decode proto bytes → verify fields match.

Integration test: connect to IB Gateway, place/cancel orders, request data — verify server accepts protobuf-encoded requests.
