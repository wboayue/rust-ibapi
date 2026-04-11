# Phase 3: Inbound Protobuf Decoding — Replace Text Decoders

## Context

All inbound messages from server version 201+ arrive as protobuf payloads. Replace the text-based `decode_*` functions (which parse `ResponseMessage` via `next_int()`, `next_string()`, etc.) with protobuf decoders that deserialize prost types and map them to existing domain types.

**Depends on:** Phase 2 (binary framing delivers `raw_bytes` for protobuf messages)

## Architecture

```
raw_bytes → prost::Message::decode() → proto::Foo → domain::Foo
                                            ↑
                               src/proto/decoders.rs (shared converters)
```

**Shared converters** in `src/proto/decoders.rs` handle common types reused across messages:
- `proto::Contract` → `contracts::Contract`
- `proto::Order` → `orders::Order`
- `proto::OrderState` → `orders::OrderState`
- `proto::ComboLeg` → `contracts::ComboLeg`
- `proto::DeltaNeutralContract` → `contracts::DeltaNeutralContract`
- `proto::SoftDollarTier` → `orders::SoftDollarTier`

**Domain decoders** in each module's `decoders.rs` call shared converters and handle message-specific logic.

## Conversion Pattern

```rust
// src/proto/decoders.rs
pub fn decode_contract(proto: &proto::Contract) -> Contract {
    Contract {
        contract_id: proto.con_id.unwrap_or_default(),
        symbol: Symbol::from(proto.symbol.clone().unwrap_or_default()),
        security_type: SecurityType::from(proto.sec_type.as_deref().unwrap_or_default()),
        // ... field-by-field mapping
    }
}

// src/orders/common/decoders.rs
pub(in crate::orders) fn decode_open_order(bytes: &[u8]) -> Result<OrderData, Error> {
    let proto_msg = proto::OpenOrder::decode(bytes)?;
    let contract = proto::decoders::decode_contract(proto_msg.contract.as_ref().unwrap_or_default());
    let order = proto::decoders::decode_order(proto_msg.order.as_ref().unwrap_or_default());
    let order_state = proto::decoders::decode_order_state(proto_msg.order_state.as_ref().unwrap_or_default());
    Ok(OrderData { order_id: proto_msg.order_id.unwrap_or_default(), contract, order, order_state })
}
```

C# reference: `EDecoderUtils.cs` — `decodeContract()`, `decodeOrder()`, `decodeOrderState()` etc.

## Domain Modules to Update

### Orders (`src/orders/common/decoders.rs`)
Current: `OrderDecoder` struct with `read_order_id()`, `read_contract_fields()`, etc. parsing `ResponseMessage`
Replace with protobuf decoders for:
- `decode_open_order(bytes)` — proto::OpenOrder
- `decode_order_status(bytes)` — proto::OrderStatus
- `decode_execution_data(bytes)` — proto::ExecutionData (via ExecutionReport or similar)
- `decode_completed_order(bytes)` — proto::CompletedOrder
- `decode_commission_report(bytes)` — proto::CommissionAndFeesReport

### Contracts (`src/contracts/common/decoders.rs`)
- `decode_contract_data(bytes)` — proto::ContractData
- `decode_bond_contract_data(bytes)` — proto::ContractData (bond variant)
- `decode_contract_data_end(bytes)` — proto::ContractDataEnd

### Market Data - Realtime (`src/market_data/realtime/common/decoders.rs`)
- `decode_tick_price(bytes)` — proto::TickPrice
- `decode_tick_size(bytes)` — proto::TickSize
- `decode_tick_string(bytes)` — proto::TickString
- `decode_tick_generic(bytes)` — proto::TickGeneric
- `decode_tick_option_computation(bytes)` — proto::TickOptionComputation
- `decode_tick_by_tick(bytes)` — proto::TickByTickData
- `decode_market_depth(bytes)` — market depth proto types

### Market Data - Historical (`src/market_data/historical/common/decoders.rs`)
- `decode_historical_data(bytes)` — proto::HistoricalData
- `decode_historical_ticks(bytes)` — proto::HistoricalTick variants
- `decode_realtime_bar(bytes)` — proto::RealTimeBarTick
- `decode_head_timestamp(bytes)` — proto::HeadTimestamp

### Accounts (`src/accounts/common/decoders.rs`)
- `decode_account_value(bytes)` — proto::AccountValue
- `decode_portfolio_value(bytes)` — proto::PortfolioValue
- `decode_position(bytes)` — proto::Position
- `decode_pnl(bytes)` — proto::PnL / proto::PnLSingle

### News (`src/news/common/decoders.rs`)
- `decode_news_bulletins(bytes)` — proto::NewsBulletins
- `decode_news_article(bytes)` — proto::NewsArticle
- `decode_historical_news(bytes)` — proto::HistoricalNews

### Scanner (`src/scanner/common/decoders.rs`)
- `decode_scanner_data(bytes)` — proto::ScannerData
- `decode_scanner_parameters(bytes)` — proto::ScannerParameters

### Display Groups (`src/display_groups/common/decoders.rs`)
- Display group response decoders

### WSH (`src/wsh/common/decoders.rs`)
- `decode_wsh_metadata(bytes)` — proto::WshMetaData
- `decode_wsh_event_data(bytes)` — proto::WshEventData

## Dispatch Changes

The transport layer currently calls decoders with `ResponseMessage`. Update the call sites:
- If `response.is_protobuf()`: call `decode_*(response.raw_bytes())`
- If not: call legacy text decoder (keep for now, remove in Phase 5)

This can be done per-domain-module in separate PRs.

## Files Modified

| File | Change |
|------|--------|
| `src/proto/mod.rs` | Add `pub mod decoders;` |
| `src/proto/decoders.rs` | (new) Shared proto→domain converters |
| `src/orders/common/decoders.rs` | Rewrite to protobuf |
| `src/contracts/common/decoders.rs` | Rewrite to protobuf |
| `src/market_data/realtime/common/decoders.rs` | Rewrite to protobuf |
| `src/market_data/historical/common/decoders.rs` | Rewrite to protobuf |
| `src/accounts/common/decoders.rs` | Rewrite to protobuf |
| `src/news/common/decoders.rs` | Rewrite to protobuf |
| `src/scanner/common/decoders.rs` | Rewrite to protobuf |
| `src/display_groups/common/decoders.rs` | Rewrite to protobuf |
| `src/wsh/common/decoders.rs` | Rewrite to protobuf |

## C# Reference

| File | What |
|------|------|
| `EDecoderUtils.cs` lines 16-42 | `decodeContract()` — primary field mapping reference |
| `EDecoderUtils.cs` lines 131-292 | `decodeOrder()` — 90+ field mappings |
| `EDecoderUtils.cs` lines 421-449 | `decodeOrderState()` |
| `EDecoder.cs` lines 2658-2689 | `OpenOrderEventProtoBuf()` — decode + convert pattern |
| `EDecoder.cs` lines 677-706 | `CompletedOrderEventProtoBuf()` |

## Verification

```bash
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features
just test
```

Unit tests: construct protobuf bytes with prost, decode via new functions, assert domain struct fields match.

Integration test: connect to IB Gateway, request contract details / market data, verify responses decode correctly.
