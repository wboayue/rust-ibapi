use crate::messages::{IncomingMessages, Notice, ResponseMessage};
use crate::orders::common::decoders;
use crate::orders::{CancelOrder, Executions, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

impl StreamDecoder<PlaceOrder> for PlaceOrder {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::OpenOrder,
        IncomingMessages::OrderStatus,
        IncomingMessages::ExecutionData,
        IncomingMessages::CommissionsReport,
        IncomingMessages::Error,
    ];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<PlaceOrder, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(PlaceOrder::OpenOrder(decoders::decode_open_order(
                context.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(PlaceOrder::OrderStatus(decoders::decode_order_status(context.server_version, message)?)),
            IncomingMessages::ExecutionData => Ok(PlaceOrder::ExecutionData(decoders::decode_execution_data(
                context.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(PlaceOrder::CommissionReport(decoders::decode_commission_report(
                context.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(PlaceOrder::Message(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<OrderUpdate> for OrderUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::OpenOrder,
        IncomingMessages::OrderStatus,
        IncomingMessages::ExecutionData,
        IncomingMessages::CommissionsReport,
        IncomingMessages::Error,
    ];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<OrderUpdate, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(OrderUpdate::OpenOrder(decoders::decode_open_order(
                context.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(OrderUpdate::OrderStatus(decoders::decode_order_status(context.server_version, message)?)),
            IncomingMessages::ExecutionData => Ok(OrderUpdate::ExecutionData(decoders::decode_execution_data(
                context.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(OrderUpdate::CommissionReport(decoders::decode_commission_report(
                context.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(OrderUpdate::Message(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<CancelOrder> for CancelOrder {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::OrderStatus, IncomingMessages::Error];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<CancelOrder, Error> {
        match message.message_type() {
            IncomingMessages::OrderStatus => Ok(CancelOrder::OrderStatus(decoders::decode_order_status(context.server_version, message)?)),
            IncomingMessages::Error => Ok(CancelOrder::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<Orders> for Orders {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::CompletedOrder,
        IncomingMessages::CommissionsReport,
        IncomingMessages::OpenOrder,
        IncomingMessages::OrderStatus,
        IncomingMessages::OpenOrderEnd,
        IncomingMessages::CompletedOrdersEnd,
        IncomingMessages::Error,
    ];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Orders, Error> {
        match message.message_type() {
            IncomingMessages::CompletedOrder => Ok(Orders::OrderData(decoders::decode_completed_order(
                context.server_version,
                message.clone(),
            )?)),
            IncomingMessages::CommissionsReport => Ok(Orders::OrderData(decoders::decode_open_order(context.server_version, message.clone())?)),
            IncomingMessages::OpenOrder => Ok(Orders::OrderData(decoders::decode_open_order(context.server_version, message.clone())?)),
            IncomingMessages::OrderStatus => Ok(Orders::OrderStatus(decoders::decode_order_status(context.server_version, message)?)),
            IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => Err(Error::EndOfStream),
            IncomingMessages::Error => Ok(Orders::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<Executions> for Executions {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::ExecutionData,
        IncomingMessages::CommissionsReport,
        IncomingMessages::ExecutionDataEnd,
        IncomingMessages::Error,
    ];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Executions, Error> {
        match message.message_type() {
            IncomingMessages::ExecutionData => Ok(Executions::ExecutionData(decoders::decode_execution_data(
                context.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(Executions::CommissionReport(decoders::decode_commission_report(
                context.server_version,
                message,
            )?)),
            IncomingMessages::ExecutionDataEnd => Err(Error::EndOfStream),
            IncomingMessages::Error => Ok(Executions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<ExerciseOptions> for ExerciseOptions {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus, IncomingMessages::Error];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<ExerciseOptions, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(ExerciseOptions::OpenOrder(decoders::decode_open_order(
                context.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(ExerciseOptions::OrderStatus(decoders::decode_order_status(
                context.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(ExerciseOptions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}
