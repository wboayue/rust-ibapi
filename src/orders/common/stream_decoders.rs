use crate::messages::{IncomingMessages, ResponseMessage};
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
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<PlaceOrder, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(PlaceOrder::OpenOrder(decoders::decode_open_order(message)?)),
            IncomingMessages::OrderStatus => Ok(PlaceOrder::OrderStatus(decoders::decode_order_status(message)?)),
            IncomingMessages::ExecutionData => Ok(PlaceOrder::ExecutionData(decoders::decode_execution_data(message)?)),
            IncomingMessages::CommissionsReport => Ok(PlaceOrder::CommissionReport(decoders::decode_commission_report(message)?)),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}

impl StreamDecoder<OrderUpdate> for OrderUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::OpenOrder,
        IncomingMessages::OrderStatus,
        IncomingMessages::ExecutionData,
        IncomingMessages::CommissionsReport,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<OrderUpdate, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(OrderUpdate::OpenOrder(decoders::decode_open_order(message)?)),
            IncomingMessages::OrderStatus => Ok(OrderUpdate::OrderStatus(decoders::decode_order_status(message)?)),
            IncomingMessages::ExecutionData => Ok(OrderUpdate::ExecutionData(decoders::decode_execution_data(message)?)),
            IncomingMessages::CommissionsReport => Ok(OrderUpdate::CommissionReport(decoders::decode_commission_report(message)?)),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}

impl StreamDecoder<CancelOrder> for CancelOrder {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::OrderStatus];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<CancelOrder, Error> {
        match message.message_type() {
            IncomingMessages::OrderStatus => Ok(CancelOrder::OrderStatus(decoders::decode_order_status(message)?)),
            _ => Err(Error::unexpected_response(message)),
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
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Orders, Error> {
        match message.message_type() {
            IncomingMessages::CompletedOrder => Ok(Orders::OrderData(decoders::decode_completed_order(message)?)),
            IncomingMessages::CommissionsReport => Ok(Orders::OrderData(decoders::decode_open_order(message)?)),
            IncomingMessages::OpenOrder => Ok(Orders::OrderData(decoders::decode_open_order(message)?)),
            IncomingMessages::OrderStatus => Ok(Orders::OrderStatus(decoders::decode_order_status(message)?)),
            IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}

impl StreamDecoder<Executions> for Executions {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::ExecutionData,
        IncomingMessages::CommissionsReport,
        IncomingMessages::ExecutionDataEnd,
    ];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Executions, Error> {
        match message.message_type() {
            IncomingMessages::ExecutionData => Ok(Executions::ExecutionData(decoders::decode_execution_data(message)?)),
            IncomingMessages::CommissionsReport => Ok(Executions::CommissionReport(decoders::decode_commission_report(message)?)),
            IncomingMessages::ExecutionDataEnd => Err(Error::EndOfStream),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}

impl StreamDecoder<ExerciseOptions> for ExerciseOptions {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::OpenOrder, IncomingMessages::OrderStatus];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<ExerciseOptions, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(ExerciseOptions::OpenOrder(decoders::decode_open_order(message)?)),
            IncomingMessages::OrderStatus => Ok(ExerciseOptions::OrderStatus(decoders::decode_order_status(message)?)),
            _ => Err(Error::unexpected_response(message)),
        }
    }
}
