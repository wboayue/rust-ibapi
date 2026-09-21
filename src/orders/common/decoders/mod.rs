use crate::messages::ResponseMessage;
use crate::orders::{CommissionReport, ExecutionData, OrderBound, OrderData, OrderStatus};
use crate::Error;

// All originating outgoing-request gates for OpenOrder, CompletedOrder,
// OrderStatus, ExecutionData, and CommissionReport are <= the connection floor.
// The server always emits proto framing for these messages; text-framed arrival
// is rejected via `ResponseMessage::require_proto`, which raises
// `Error::UnexpectedWireFormat` (docs/rules/wire/proto-only-decoding.md).

/// A proto submessage or scalar the frame is meaningless without.
///
/// The reference client drops the whole frame when one is absent —
/// `EDecoder.cs`'s `OpenOrderEventProtoBuf` returns before `eWrapper.openOrder(..)`
/// if `Contract`, `Order` or `OrderState` is null, rather than synthesizing a
/// default. This crate has no "skip this frame" channel, so it surfaces the
/// malformed frame as `Error::Parse` instead. Defaulting is the one option
/// neither client takes: it hands the caller a phantom BUY order over an empty
/// contract, which reads as real data (docs/rules/wire/enum-typing.md).
fn required<T>(field: Option<T>, name: &str, message: &str) -> Result<T, Error> {
    field.ok_or_else(|| Error::parse_proto(name, format!("missing in {message}")))
}

pub(crate) fn decode_open_order(message: &ResponseMessage) -> Result<OrderData, Error> {
    decode_open_order_proto(message.require_proto()?)
}

pub(crate) fn decode_order_status(message: &ResponseMessage) -> Result<OrderStatus, Error> {
    decode_order_status_proto(message.require_proto()?)
}

pub(crate) fn decode_order_bound(message: &ResponseMessage) -> Result<OrderBound, Error> {
    let p: crate::proto::OrderBound = prost::Message::decode(message.require_proto()?)?;
    Ok(OrderBound {
        perm_id: required(p.perm_id, "perm_id", "OrderBound")?,
        client_id: required(p.client_id, "client_id", "OrderBound")?,
        order_id: required(p.order_id, "order_id", "OrderBound")?,
    })
}

pub(crate) fn decode_execution_data(message: &ResponseMessage) -> Result<ExecutionData, Error> {
    decode_execution_data_proto(message.require_proto()?)
}

pub(crate) fn decode_commission_report(message: &ResponseMessage) -> Result<CommissionReport, Error> {
    decode_commission_report_proto(message.require_proto()?)
}

pub(crate) fn decode_completed_order(message: &ResponseMessage) -> Result<OrderData, Error> {
    decode_completed_order_proto(message.require_proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_open_order_proto(bytes: &[u8]) -> Result<OrderData, Error> {
    let p: crate::proto::OpenOrder = prost::Message::decode(bytes)?;

    Ok(OrderData {
        order_id: p.order_id.unwrap_or_default(),
        contract: crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "OpenOrder")?)?,
        order: crate::proto::decoders::decode_order(required(p.order.as_ref(), "order", "OpenOrder")?)?,
        order_state: crate::proto::decoders::decode_order_state(required(p.order_state.as_ref(), "order_state", "OpenOrder")?)?,
    })
}

pub(crate) fn decode_order_status_proto(bytes: &[u8]) -> Result<OrderStatus, Error> {
    let p: crate::proto::OrderStatus = prost::Message::decode(bytes)?;

    Ok(OrderStatus {
        order_id: p.order_id.unwrap_or_default(),
        status: crate::proto::decoders::parse_required(p.status.as_deref(), "OrderStatus")?,
        filled: crate::proto::decoders::parse_decimal_or_zero(p.filled.as_deref())?,
        remaining: crate::proto::decoders::parse_decimal_or_zero(p.remaining.as_deref())?,
        average_fill_price: p.avg_fill_price,
        perm_id: p.perm_id.unwrap_or_default(),
        parent_id: p.parent_id.unwrap_or_default(),
        last_fill_price: p.last_fill_price,
        client_id: p.client_id.unwrap_or_default(),
        why_held: p.why_held.unwrap_or_default(),
        market_cap_price: p.mkt_cap_price,
    })
}

pub(crate) fn decode_execution_data_proto(bytes: &[u8]) -> Result<ExecutionData, Error> {
    let p: crate::proto::ExecutionDetails = prost::Message::decode(bytes)?;

    Ok(ExecutionData {
        request_id: p.req_id.unwrap_or_default(),
        contract: crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "ExecutionDetails")?)?,
        execution: crate::proto::decoders::decode_execution(required(p.execution.as_ref(), "execution", "ExecutionDetails")?)?,
    })
}

pub(crate) fn decode_completed_order_proto(bytes: &[u8]) -> Result<OrderData, Error> {
    let p: crate::proto::CompletedOrder = prost::Message::decode(bytes)?;
    let contract = crate::proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "CompletedOrder")?)?;
    let order = crate::proto::decoders::decode_order(required(p.order.as_ref(), "order", "CompletedOrder")?)?;
    let order_state = crate::proto::decoders::decode_order_state(required(p.order_state.as_ref(), "order_state", "CompletedOrder")?)?;

    Ok(OrderData {
        // Completed orders carry no live order_id; preserve the legacy text-decoder
        // sentinel so v2 → v3 consumers see the same value.
        order_id: -1,
        contract,
        order,
        order_state,
    })
}

pub(crate) fn decode_commission_report_proto(bytes: &[u8]) -> Result<CommissionReport, Error> {
    let p: crate::proto::CommissionAndFeesReport = prost::Message::decode(bytes)?;

    Ok(CommissionReport {
        execution_id: p.exec_id.unwrap_or_default(),
        commission: p.commission_and_fees.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
        realized_pnl: crate::proto::decoders::optional_f64(p.realized_pnl),
        yields: crate::proto::decoders::optional_f64(p.bond_yield),
        yield_redemption_date: p.yield_redemption_date.unwrap_or_default(),
    })
}

pub(crate) fn decode_next_valid_id_proto(p: crate::proto::NextValidId) -> Result<i32, Error> {
    Ok(p.order_id.unwrap_or_default())
}

#[cfg(test)]
mod tests;
