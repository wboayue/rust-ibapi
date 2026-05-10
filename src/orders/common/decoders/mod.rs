use crate::messages::ResponseMessage;
use crate::orders::{CommissionReport, ExecutionData, OrderData, OrderStatus};
use crate::Error;

// All originating outgoing-request gates for OpenOrder, CompletedOrder,
// OrderStatus, ExecutionData, and CommissionReport are <= the connection floor
// (`PROTOBUF_SCAN_DATA` = 210). The server always emits proto framing for these
// messages; text-framed arrival is rejected via `ResponseMessage::require_proto`
// and skip-classifies (rule 20).

pub(crate) fn decode_open_order(message: &mut ResponseMessage) -> Result<OrderData, Error> {
    decode_open_order_proto(message.require_proto()?)
}

pub(crate) fn decode_order_status(message: &mut ResponseMessage) -> Result<OrderStatus, Error> {
    decode_order_status_proto(message.require_proto()?)
}

pub(crate) fn decode_execution_data(message: &mut ResponseMessage) -> Result<ExecutionData, Error> {
    decode_execution_data_proto(message.require_proto()?)
}

pub(crate) fn decode_commission_report(message: &mut ResponseMessage) -> Result<CommissionReport, Error> {
    decode_commission_report_proto(message.require_proto()?)
}

pub(crate) fn decode_completed_order(message: &mut ResponseMessage) -> Result<OrderData, Error> {
    decode_completed_order_proto(message.require_proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_open_order_proto(bytes: &[u8]) -> Result<OrderData, Error> {
    let p: crate::proto::OpenOrder = prost::Message::decode(bytes)?;
    let contract = p.contract.as_ref().map(crate::proto::decoders::decode_contract).unwrap_or_default();
    let order = p.order.as_ref().map(crate::proto::decoders::decode_order).unwrap_or_default();
    let order_state = p
        .order_state
        .as_ref()
        .map(crate::proto::decoders::decode_order_state)
        .transpose()?
        .unwrap_or_default();

    Ok(OrderData {
        order_id: p.order_id.unwrap_or_default(),
        contract,
        order,
        order_state,
    })
}

pub(crate) fn decode_order_status_proto(bytes: &[u8]) -> Result<OrderStatus, Error> {
    let p: crate::proto::OrderStatus = prost::Message::decode(bytes)?;

    Ok(OrderStatus {
        order_id: p.order_id.unwrap_or_default(),
        status: crate::proto::decoders::parse_order_status(&p.status)?,
        filled: crate::proto::decoders::parse_f64(&p.filled),
        remaining: crate::proto::decoders::parse_f64(&p.remaining),
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
        contract: p.contract.as_ref().map(crate::proto::decoders::decode_contract).unwrap_or_default(),
        execution: p.execution.as_ref().map(crate::proto::decoders::decode_execution).unwrap_or_default(),
    })
}

pub(crate) fn decode_completed_order_proto(bytes: &[u8]) -> Result<OrderData, Error> {
    let p: crate::proto::CompletedOrder = prost::Message::decode(bytes)?;
    let contract = p.contract.as_ref().map(crate::proto::decoders::decode_contract).unwrap_or_default();
    let order = p.order.as_ref().map(crate::proto::decoders::decode_order).unwrap_or_default();
    let order_state = p
        .order_state
        .as_ref()
        .map(crate::proto::decoders::decode_order_state)
        .transpose()?
        .unwrap_or_default();

    Ok(OrderData {
        order_id: order.order_id,
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

pub(crate) fn decode_next_valid_id(message: &mut ResponseMessage) -> Result<i32, Error> {
    message.decode_proto_or_text(
        |bytes| {
            let p: crate::proto::NextValidId = prost::Message::decode(bytes)?;
            Ok(p.order_id.unwrap_or_default())
        },
        |msg| {
            // text fields: [msg_type, version, order_id]
            msg.peek_int(2)
        },
    )
}

#[cfg(test)]
mod tests;
