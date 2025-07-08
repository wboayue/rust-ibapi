use super::common::{decoders, encoders, verify};
use super::{
    CancelOrder, ExerciseAction, ExerciseOptions, ExecutionFilter, Executions,
    OrderUpdate, Orders, PlaceOrder,
};
use crate::client::{DataStream, ResponseContext, Subscription};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, Notice, OutgoingMessages, ResponseMessage};
use crate::{server_versions, Client, Error};
use time::OffsetDateTime;

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;

impl DataStream<PlaceOrder> for PlaceOrder {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<PlaceOrder, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(PlaceOrder::OpenOrder(decoders::decode_open_order(
                client.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(PlaceOrder::OrderStatus(decoders::decode_order_status(client.server_version, message)?)),
            IncomingMessages::ExecutionData => Ok(PlaceOrder::ExecutionData(decoders::decode_execution_data(
                client.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(PlaceOrder::CommissionReport(decoders::decode_commission_report(
                client.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(PlaceOrder::Message(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl DataStream<OrderUpdate> for OrderUpdate {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<OrderUpdate, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(OrderUpdate::OpenOrder(decoders::decode_open_order(
                client.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(OrderUpdate::OrderStatus(decoders::decode_order_status(client.server_version, message)?)),
            IncomingMessages::ExecutionData => Ok(OrderUpdate::ExecutionData(decoders::decode_execution_data(
                client.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(OrderUpdate::CommissionReport(decoders::decode_commission_report(
                client.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(OrderUpdate::Message(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl DataStream<CancelOrder> for CancelOrder {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<CancelOrder, Error> {
        match message.message_type() {
            IncomingMessages::OrderStatus => Ok(CancelOrder::OrderStatus(decoders::decode_order_status(client.server_version, message)?)),
            IncomingMessages::Error => Ok(CancelOrder::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl DataStream<Orders> for Orders {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Orders, Error> {
        match message.message_type() {
            IncomingMessages::CompletedOrder => Ok(Orders::OrderData(decoders::decode_completed_order(
                client.server_version,
                message.clone(),
            )?)),
            IncomingMessages::CommissionsReport => Ok(Orders::OrderData(decoders::decode_open_order(client.server_version, message.clone())?)),
            IncomingMessages::OpenOrder => Ok(Orders::OrderData(decoders::decode_open_order(client.server_version, message.clone())?)),
            IncomingMessages::OrderStatus => Ok(Orders::OrderStatus(decoders::decode_order_status(client.server_version, message)?)),
            IncomingMessages::OpenOrderEnd | IncomingMessages::CompletedOrdersEnd => Err(Error::EndOfStream),
            IncomingMessages::Error => Ok(Orders::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl DataStream<Executions> for Executions {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Executions, Error> {
        match message.message_type() {
            IncomingMessages::ExecutionData => Ok(Executions::ExecutionData(decoders::decode_execution_data(
                client.server_version,
                message,
            )?)),
            IncomingMessages::CommissionsReport => Ok(Executions::CommissionReport(decoders::decode_commission_report(
                client.server_version,
                message,
            )?)),
            IncomingMessages::ExecutionDataEnd => Err(Error::EndOfStream),
            IncomingMessages::Error => Ok(Executions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl DataStream<ExerciseOptions> for ExerciseOptions {
    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<ExerciseOptions, Error> {
        match message.message_type() {
            IncomingMessages::OpenOrder => Ok(ExerciseOptions::OpenOrder(decoders::decode_open_order(
                client.server_version,
                message.clone(),
            )?)),
            IncomingMessages::OrderStatus => Ok(ExerciseOptions::OrderStatus(decoders::decode_order_status(
                client.server_version,
                message,
            )?)),
            IncomingMessages::Error => Ok(ExerciseOptions::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

/// Subscribes to order update events. Only one subscription can be active at a time.
///
/// This function returns a subscription that will receive updates of activity for all orders placed by the client.
pub fn order_update_stream<'a>(client: &'a Client) -> Result<Subscription<'a, OrderUpdate>, Error> {
    let subscription = client.create_order_update_subscription()?;
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

/// Submits an Order.
///
/// After the order is submitted correctly, events will be returned concerning the order's activity.
/// This is a fire-and-forget method that does not wait for confirmation or return a subscription.
///
/// # Arguments
/// * `client` - The client instance
/// * `order_id` - Unique order identifier
/// * `contract` - Contract to submit order for
/// * `order` - Order details
///
/// # Returns
/// * `Ok(())` if the order was successfully sent
/// * `Err(Error)` if validation failed or sending failed
///
/// # See Also
/// * [TWS API Documentation](https://interactivebrokers.github.io/tws-api/order_submission.html)
pub fn submit_order(client: &Client, order_id: i32, contract: &Contract, order: &super::Order) -> Result<(), Error> {
    verify::verify_order(client, order, order_id)?;
    verify::verify_order_contract(client, contract, order_id)?;

    let request = encoders::encode_place_order(client.server_version(), order_id, contract, order)?;
    client.send_message(request)?;

    Ok(())
}

// Submits an Order.
// After the order is submitted correctly, events will be returned concerning the order's activity.
// https://interactivebrokers.github.io/tws-api/order_submission.html
pub fn place_order<'a>(client: &'a Client, order_id: i32, contract: &Contract, order: &super::Order) -> Result<Subscription<'a, PlaceOrder>, Error> {
    verify::verify_order(client, order, order_id)?;
    verify::verify_order_contract(client, contract, order_id)?;

    let request = encoders::encode_place_order(client.server_version(), order_id, contract, order)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Cancels an open [Order].
pub fn cancel_order<'a>(client: &'a Client, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<'a, CancelOrder>, Error> {
    if !manual_order_cancel_time.is_empty() {
        client.check_server_version(
            server_versions::MANUAL_ORDER_TIME,
            "It does not support manual order cancel time attribute",
        )?
    }

    let request = encoders::encode_cancel_order(client.server_version(), order_id, manual_order_cancel_time)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Cancels all open [Order]s.
pub fn global_cancel(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel requests.")?;

    let message = encoders::encode_global_cancel()?;

    let request_id = client.next_request_id();
    client.send_order(request_id, message)?;

    Ok(())
}

// Gets next valid order id
pub fn next_valid_order_id(client: &Client) -> Result<i32, Error> {
    let message = encoders::encode_next_valid_order_id()?;

    let subscription = client.send_shared_request(OutgoingMessages::RequestIds, message)?;

    if let Some(Ok(message)) = subscription.next() {
        let order_id_index = 2;
        let next_order_id = message.peek_int(order_id_index)?;

        client.set_next_order_id(next_order_id);

        Ok(next_order_id)
    } else {
        Err(Error::Simple("no response from server".into()))
    }
}

// Requests completed [Order]s.
pub fn completed_orders(client: &Client, api_only: bool) -> Result<Subscription<Orders>, Error> {
    client.check_server_version(server_versions::COMPLETED_ORDERS, "It does not support completed orders requests.")?;

    let request = encoders::encode_completed_orders(api_only)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestCompletedOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

/// Requests all open orders places by this specific API client (identified by the API client id).
/// For client ID 0, this will bind previous manual TWS orders.
///
/// # Arguments
/// * `client` - [Client] used to communicate with server.
///
pub fn open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests all *current* open orders in associated accounts at the current moment.
// Open orders are returned once; this function does not initiate a subscription.
pub fn all_open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_all_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
pub fn auto_open_orders(client: &Client, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_auto_open_orders(auto_bind)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

// Requests current day's (since midnight) executions matching the filter.
//
// Only the current day's executions can be retrieved.
// Along with the [ExecutionData], the [CommissionReport] will also be returned.
// When requesting executions, a filter can be specified to receive only a subset of them
//
// # Arguments
// * `filter` - filter criteria used to determine which execution reports are returned
pub fn executions(client: &Client, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
    let request_id = client.next_request_id();

    let request = encoders::encode_executions(client.server_version(), request_id, &filter)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

pub fn exercise_options<'a>(
    client: &'a Client,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<Subscription<'a, ExerciseOptions>, Error> {
    let request_id = client.next_request_id();

    let request = encoders::encode_exercise_options(
        client.server_version(),
        request_id,
        contract,
        exercise_action,
        exercise_quantity,
        account,
        ovrd,
        manual_order_time,
    )?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}