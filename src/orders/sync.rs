use std::sync::Arc;

use super::common::{decoders, encoders, verify};
use super::{CancelOrder, ExecutionFilter, Executions, ExerciseAction, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
use crate::client::blocking::Subscription;
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, Notice, OutgoingMessages, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::{client::sync::Client, server_versions, Error};
use time::OffsetDateTime;

impl StreamDecoder<PlaceOrder> for PlaceOrder {
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
    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<CancelOrder, Error> {
        match message.message_type() {
            IncomingMessages::OrderStatus => Ok(CancelOrder::OrderStatus(decoders::decode_order_status(context.server_version, message)?)),
            IncomingMessages::Error => Ok(CancelOrder::Notice(Notice::from(message))),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl StreamDecoder<Orders> for Orders {
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

/// Subscribes to order update events. Only one subscription can be active at a time.
///
/// This function returns a subscription that will receive updates of activity for all orders placed by the client.
pub(crate) fn order_update_stream(client: &Client) -> Result<Subscription<OrderUpdate>, Error> {
    let subscription = client.create_order_update_subscription()?;
    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
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
pub(crate) fn submit_order(client: &Client, order_id: i32, contract: &Contract, order: &super::Order) -> Result<(), Error> {
    verify::verify_order(client, order, order_id)?;
    verify::verify_order_contract(client, contract, order_id)?;

    let request = encoders::encode_place_order(client.server_version, order_id, contract, order)?;
    client.send_message(request)?;

    Ok(())
}

/// Submits an Order and returns a subscription to receive order events.
///
/// After the order is submitted correctly, events will be returned concerning the order's activity.
///
/// # Arguments
/// * `client` - The client instance
/// * `order_id` - Unique order identifier
/// * `contract` - Contract to submit order for
/// * `order` - Order details
///
/// # Returns
/// * `Ok(Subscription<PlaceOrder>)` - Subscription to receive order events
/// * `Err(Error)` - If validation failed or sending failed
///
/// # See Also
/// * [TWS API Documentation](https://interactivebrokers.github.io/tws-api/order_submission.html)
pub(crate) fn place_order(client: &Client, order_id: i32, contract: &Contract, order: &super::Order) -> Result<Subscription<PlaceOrder>, Error> {
    verify::verify_order(client, order, order_id)?;
    verify::verify_order_contract(client, contract, order_id)?;

    let request = encoders::encode_place_order(client.server_version, order_id, contract, order)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Cancels an open Order and returns a subscription to receive cancellation events.
///
/// # Arguments
/// * `client` - The client instance
/// * `order_id` - Order identifier to cancel
/// * `manual_order_cancel_time` - Manual cancellation time (empty string for immediate)
///
/// # Returns
/// * `Ok(Subscription<CancelOrder>)` - Subscription to receive cancellation status
/// * `Err(Error)` - If the request failed
pub(crate) fn cancel_order(client: &Client, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
    if !manual_order_cancel_time.is_empty() {
        client.check_server_version(
            server_versions::MANUAL_ORDER_TIME,
            "It does not support manual order cancel time attribute",
        )?
    }

    let request = encoders::encode_cancel_order(client.server_version, order_id, manual_order_cancel_time)?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Cancels all open Orders.
///
/// # Arguments
/// * `client` - The client instance
///
/// # Returns
/// * `Ok(())` - If the cancel request was successfully sent
/// * `Err(Error)` - If the server version doesn't support global cancel or sending failed
pub(crate) fn global_cancel(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel requests.")?;

    let message = encoders::encode_global_cancel()?;
    client.send_message(message)?;

    Ok(())
}

/// Gets the next valid order ID from the server.
///
/// # Arguments
/// * `client` - The client instance
///
/// # Returns
/// * `Ok(i32)` - The next valid order ID
/// * `Err(Error)` - If the request failed or no response received
pub(crate) fn next_valid_order_id(client: &Client) -> Result<i32, Error> {
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

/// Requests completed Orders.
///
/// # Arguments
/// * `client` - The client instance
/// * `api_only` - If true, only return orders placed by this API client
///
/// # Returns
/// * `Ok(Subscription<Orders>)` - Subscription to receive completed orders
/// * `Err(Error)` - If the server version doesn't support this feature or request failed
pub(crate) fn completed_orders(client: &Client, api_only: bool) -> Result<Subscription<Orders>, Error> {
    client.check_server_version(server_versions::COMPLETED_ORDERS, "It does not support completed orders requests.")?;

    let request = encoders::encode_completed_orders(api_only)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestCompletedOrders, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Requests all open orders places by this specific API client (identified by the API client id).
/// For client ID 0, this will bind previous manual TWS orders.
///
/// # Arguments
/// * `client` - [Client] used to communicate with server.
///
pub(crate) fn open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestOpenOrders, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Requests all current open orders in associated accounts at the current moment.
///
/// Open orders are returned once; this function does not initiate a live subscription.
///
/// # Arguments
/// * `client` - The client instance
///
/// # Returns
/// * `Ok(Subscription<Orders>)` - Subscription to receive all open orders
/// * `Err(Error)` - If the request failed
pub(crate) fn all_open_orders(client: &Client) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_all_open_orders()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Requests status updates about future orders placed from TWS.
///
/// Can only be used with client ID 0.
///
/// # Arguments
/// * `client` - The client instance (must have client ID 0)
/// * `auto_bind` - If true, bind to orders placed from TWS
///
/// # Returns
/// * `Ok(Subscription<Orders>)` - Subscription to receive order updates
/// * `Err(Error)` - If the request failed
pub(crate) fn auto_open_orders(client: &Client, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
    let request = encoders::encode_auto_open_orders(auto_bind)?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Requests current day's (since midnight) executions matching the filter.
///
/// Only the current day's executions can be retrieved.
/// Along with the ExecutionData, the CommissionReport will also be returned.
/// When requesting executions, a filter can be specified to receive only a subset of them.
///
/// # Arguments
/// * `client` - The client instance
/// * `filter` - Filter criteria used to determine which execution reports are returned
///
/// # Returns
/// * `Ok(Subscription<Executions>)` - Subscription to receive executions
/// * `Err(Error)` - If the request failed
pub(crate) fn executions(client: &Client, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
    let request_id = client.next_request_id();

    let request = encoders::encode_executions(client.server_version, request_id, &filter)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

/// Exercises options contracts.
///
/// # Arguments
/// * `client` - The client instance
/// * `contract` - Options contract to exercise
/// * `exercise_action` - Exercise action (exercise or lapse)
/// * `exercise_quantity` - Number of contracts to exercise
/// * `account` - Account identifier
/// * `ovrd` - Override system defaults
/// * `manual_order_time` - Manual order time for order placement
///
/// # Returns
/// * `Ok(Subscription<ExerciseOptions>)` - Subscription to receive exercise events
/// * `Err(Error)` - If the request failed
pub(crate) fn exercise_options(
    client: &Client,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<Subscription<ExerciseOptions>, Error> {
    let order_id = client.next_order_id();

    let request = encoders::encode_exercise_options(
        client.server_version,
        order_id,
        contract,
        exercise_action,
        exercise_quantity,
        account,
        ovrd,
        manual_order_time,
    )?;
    let subscription = client.send_order(order_id, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use crate::contracts::{ComboLeg, Contract, Currency, Exchange, SecurityType, Symbol};
    use crate::orders::conditions::TriggerMethod;
    use crate::orders::{Action, Liquidity, OcaType, OrderOrigin, ShortSaleSlot, TimeInForce};
    use crate::stubs::MessageBusStub;

    use super::*;
    use crate::orders::common::order_builder;

    #[test]
    fn place_order() {
        let message_bus = Arc::new(MessageBusStub::with_responses(vec![
            "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|1376327563|0|0|0||1376327563.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|PreSubmitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
            "3|13|PreSubmitted|0|100|0|1376327563|0|0|100||0||".to_owned(),
            "11|-1|13|76792991|TSLA|STK||0.0|||ISLAND|USD|TSLA|NMS|00025b46.63f8f39c.01.01|20230224  12:04:56|DU1234567|ISLAND|BOT|100|196.52|1376327563|100|0|100|196.52|||||2||".to_owned(),
            "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|1376327563|0|0|0||1376327563.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
            "3|13|Filled|100|0|196.52|1376327563|0|196.52|100||0||".to_owned(),
            "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|1376327563|0|0|0||1376327563.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Filled|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.0|||USD||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
            "59|1|00025b46.63f8f39c.01.01|1.0|USD|1.7976931348623157E308|1.7976931348623157E308|||".to_owned(),
        ]));

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract {
            symbol: Symbol::from("TSLA"),
            security_type: SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Contract::default()
        };

        let order_id = 13;
        let order = order_builder::market_order(Action::Buy, 100.0);

        let result = client.place_order(order_id, &contract, &order);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(
            request_messages[0].encode().replace('\0', "|"),
            "3|13|0|TSLA|STK||0|||SMART||USD|||||BUY|100|MKT|||DAY||||0||1|0|0|0|0|0|0|0||0||||||||0||-1|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
        );

        assert!(result.is_ok(), "failed to place order: {}", result.err().unwrap());

        let notifications = result.unwrap();

        if let Some(PlaceOrder::OpenOrder(open_order)) = notifications.next() {
            assert_eq!(open_order.order_id, 13, "open_order.order_id");

            let contract = &open_order.contract;
            let order = &open_order.order;
            let order_state = &open_order.order_state;

            assert_eq!(contract.contract_id, 76792991, "contract.contract_id");
            assert_eq!(contract.symbol, Symbol::from("TSLA"), "contract.symbol");
            assert_eq!(contract.security_type, SecurityType::Stock, "contract.security_type");
            assert_eq!(
                contract.last_trade_date_or_contract_month, "",
                "contract.last_trade_date_or_contract_month"
            );
            assert_eq!(contract.strike, 0.0, "contract.strike");
            assert_eq!(contract.right, "?", "contract.right");
            assert_eq!(contract.multiplier, "", "contract.multiplier");
            assert_eq!(contract.exchange, Exchange::from("SMART"), "contract.exchange");
            assert_eq!(contract.currency, Currency::from("USD"), "contract.currency");
            assert_eq!(contract.local_symbol, "TSLA", "contract.local_symbol");
            assert_eq!(contract.trading_class, "NMS", "contract.trading_class");

            assert_eq!(order.order_id, 13, "order.order_id");
            assert_eq!(order.action, Action::Buy, "order.action");
            assert_eq!(order.total_quantity, 100.0, "order.total_quantity");
            assert_eq!(order.order_type, "MKT", "order.order_type");
            assert_eq!(order.limit_price, Some(0.0), "order.limit_price");
            assert_eq!(order.aux_price, Some(0.0), "order.aux_price");
            assert_eq!(order.tif, TimeInForce::Day, "order.tif");
            assert_eq!(order.oca_group, "", "order.oca_group");
            assert_eq!(order.account, "DU1234567", "order.account");
            assert_eq!(order.open_close, None, "order.open_close");
            assert_eq!(order.origin, OrderOrigin::Customer, "order.origin");
            assert_eq!(order.order_ref, "", "order.order_ref");
            assert_eq!(order.client_id, 100, "order.client_id");
            assert_eq!(order.perm_id, 1376327563, "order.perm_id");
            assert_eq!(order.outside_rth, false, "order.outside_rth");
            assert_eq!(order.hidden, false, "order.hidden");
            assert_eq!(order.discretionary_amt, 0.0, "order.discretionary_amt");
            assert_eq!(order.good_after_time, "", "order.good_after_time");
            assert_eq!(order.fa_group, "", "order.fa_group");
            assert_eq!(order.fa_method, "", "order.fa_method");
            assert_eq!(order.fa_percentage, "", "order.fa_percentage");
            assert_eq!(order.fa_profile, "", "order.fa_profile");
            assert_eq!(order.model_code, "", "order.model_code");
            assert_eq!(order.good_till_date, "", "order.good_till_date");
            assert_eq!(order.rule_80_a, None, "order.rule_80_a");
            assert_eq!(order.percent_offset, None, "order.percent_offset");
            assert_eq!(order.settling_firm, "", "order.settling_firm");
            assert_eq!(order.short_sale_slot, ShortSaleSlot::None, "order.short_sale_slot");
            assert_eq!(order.designated_location, "", "order.designated_location");
            assert_eq!(order.exempt_code, -1, "order.exempt_code");
            assert_eq!(order.auction_strategy, None, "order.auction_strategy");
            assert_eq!(order.starting_price, None, "order.starting_price");
            assert_eq!(order.stock_ref_price, None, "order.stock_ref_price");
            assert_eq!(order.delta, None, "order.delta");
            assert_eq!(order.stock_range_lower, None, "order.stock_range_lower");
            assert_eq!(order.stock_range_upper, None, "order.stock_range_upper");
            assert_eq!(order.display_size, None, "order.display_size");
            assert_eq!(order.block_order, false, "order.block_order");
            assert_eq!(order.sweep_to_fill, false, "order.sweep_to_fill");
            assert_eq!(order.all_or_none, false, "order.all_or_none");
            assert_eq!(order.min_qty, None, "order.min_qty");
            assert_eq!(order.oca_type, OcaType::ReduceWithoutBlock, "order.oca_type");
            assert_eq!(order.parent_id, 0, "order.parent_id");
            assert_eq!(order.trigger_method, TriggerMethod::Default, "order.trigger_method");
            assert_eq!(order.volatility, None, "order.volatility");
            assert_eq!(order.volatility_type, None, "order.volatility_type");
            assert_eq!(order.delta_neutral_order_type, "None", "order.delta_neutral_order_type");
            assert_eq!(order.delta_neutral_aux_price, None, "order.delta_neutral_aux_price");
            assert_eq!(order.delta_neutral_con_id, 0, "order.delta_neutral_con_id");
            assert_eq!(order.delta_neutral_settling_firm, "", "order.delta_neutral_settling_firm");
            assert_eq!(order.delta_neutral_clearing_account, "", "order.delta_neutral_clearing_account");
            assert_eq!(order.delta_neutral_clearing_intent, "", "order.delta_neutral_clearing_intent");
            assert_eq!(order.delta_neutral_open_close, "?", "order.delta_neutral_open_close");
            assert_eq!(order.delta_neutral_short_sale, false, "order.delta_neutral_short_sale");
            assert_eq!(order.delta_neutral_short_sale_slot, 0, "order.delta_neutral_short_sale_slot");
            assert_eq!(order.delta_neutral_designated_location, "", "order.delta_neutral_designated_location");
            assert_eq!(order.continuous_update, false, "order.continuous_update");
            assert_eq!(order.reference_price_type, None, "order.reference_price_type");
            assert_eq!(order.trail_stop_price, None, "order.trail_stop_price");
            assert_eq!(order.trailing_percent, None, "order.trailing_percent");
            assert_eq!(order.basis_points, None, "order.basis_points");
            assert_eq!(order.basis_points_type, None, "order.basis_points_type");
            assert_eq!(contract.combo_legs_description, "", "contract.combo_legs_description");
            assert_eq!(contract.combo_legs.len(), 0, "contract.combo_legs.len()");
            assert_eq!(order.order_combo_legs.len(), 0, "order.order_combo_legs.len()");
            assert_eq!(order.smart_combo_routing_params.len(), 0, "order.smart_combo_routing_params.len()");
            assert_eq!(order.scale_init_level_size, None, "order.scale_init_level_size");
            assert_eq!(order.scale_subs_level_size, None, "order.scale_subs_level_size");
            assert_eq!(order.scale_price_increment, None, "order.scale_price_increment");
            assert_eq!(order.hedge_type, "", "order.hedge_type");
            assert_eq!(order.opt_out_smart_routing, false, "order.opt_out_smart_routing");
            assert_eq!(order.clearing_account, "", "order.clearing_account");
            assert_eq!(order.clearing_intent, "IB", "order.clearing_intent");
            assert_eq!(order.not_held, false, "order.not_held");
            assert_eq!(order.algo_strategy, "", "order.algo_strategy");
            assert_eq!(order.algo_params.len(), 0, "order.algo_params.len()");
            assert_eq!(order.solicited, false, "order.solicited");
            assert_eq!(order.what_if, false, "order.what_if");
            assert_eq!(order_state.status, "PreSubmitted", "order_state.status");
            assert_eq!(order_state.initial_margin_before, None, "order_state.initial_margin_before");
            assert_eq!(order_state.maintenance_margin_before, None, "order_state.maintenance_margin_before");
            assert_eq!(order_state.equity_with_loan_before, None, "order_state.equity_with_loan_before");
            assert_eq!(order_state.initial_margin_change, None, "order_state.initial_margin_change");
            assert_eq!(order_state.maintenance_margin_change, None, "order_state.maintenance_margin_change");
            assert_eq!(order_state.equity_with_loan_change, None, "order_state.equity_with_loan_change");
            assert_eq!(order_state.initial_margin_after, None, "order_state.initial_margin_after");
            assert_eq!(order_state.maintenance_margin_after, None, "order_state.maintenance_margin_after");
            assert_eq!(order_state.equity_with_loan_after, None, "order_state.equity_with_loan_after");
            assert_eq!(order_state.commission, None, "order_state.commission");
            assert_eq!(order_state.minimum_commission, None, "order_state.minimum_commission");
            assert_eq!(order_state.maximum_commission, None, "order_state.maximum_commission");
            assert_eq!(order_state.commission_currency, "", "order_state.commission_currency");
            assert_eq!(order_state.warning_text, "", "order_state.warning_text");
            assert_eq!(order.randomize_size, false, "order.randomize_size");
            assert_eq!(order.randomize_price, false, "order.randomize_price");
            assert_eq!(order.conditions.len(), 0, "order.conditions.len()");
            assert_eq!(order.adjusted_order_type, "None", "order.adjusted_order_type");
            assert_eq!(order.trigger_price, None, "order.trigger_price");
            assert_eq!(order.trail_stop_price, None, "order.trail_stop_price");
            assert_eq!(order.limit_price_offset, None, "order.lmt_price_offset");
            assert_eq!(order.adjusted_stop_price, None, "order.adjusted_stop_price");
            assert_eq!(order.adjusted_stop_limit_price, None, "order.adjusted_stop_limit_price");
            assert_eq!(order.adjusted_trailing_amount, None, "order.adjusted_trailing_amount");
            assert_eq!(order.adjustable_trailing_unit, 0, "order.adjustable_trailing_unit");
            assert_eq!(order.soft_dollar_tier.name, "", "order.soft_dollar_tier.name");
            assert_eq!(order.soft_dollar_tier.value, "", "order.soft_dollar_tier.value");
            assert_eq!(order.soft_dollar_tier.display_name, "", "order.soft_dollar_tier.display_name");
            assert_eq!(order.cash_qty, Some(0.0), "order.cash_qty");
            assert_eq!(order.dont_use_auto_price_for_hedge, true, "order.dont_use_auto_price_for_hedge");
            assert_eq!(order.is_oms_container, false, "order.is_oms_container");
            assert_eq!(order.discretionary_up_to_limit_price, false, "order.discretionary_up_to_limit_price");
            assert_eq!(order.use_price_mgmt_algo, false, "order.use_price_mgmt_algo");
            assert_eq!(order.duration, None, "order.duration");
            assert_eq!(order.post_to_ats, None, "order.post_to_ats");
            assert_eq!(order.auto_cancel_parent, false, "order.auto_cancel_parent");
            assert_eq!(order.min_trade_qty, None, "order.min_trade_qty");
            assert_eq!(order.min_compete_size, None, "order.min_compete_size");
            assert_eq!(order.compete_against_best_offset, None, "order.compete_against_best_offset");
            assert_eq!(order.mid_offset_at_whole, None, "order.mid_offset_at_whole");
            assert_eq!(order.mid_offset_at_half, None, "order.mid_offset_at_half");
        } else {
            assert!(false, "message[0] expected an open order notification");
        }

        if let Some(PlaceOrder::OrderStatus(order_status)) = notifications.next() {
            assert_eq!(order_status.order_id, 13, "order_status.order_id");
            assert_eq!(order_status.status, "PreSubmitted", "order_status.status");
            assert_eq!(order_status.filled, 0.0, "order_status.filled");
            assert_eq!(order_status.remaining, 100.0, "order_status.remaining");
            assert_eq!(order_status.average_fill_price, 0.0, "order_status.average_fill_price");
            assert_eq!(order_status.perm_id, 1376327563, "order_status.perm_id");
            assert_eq!(order_status.parent_id, 0, "order_status.parent_id");
            assert_eq!(order_status.last_fill_price, 0.0, "order_status.last_fill_price");
            assert_eq!(order_status.client_id, 100, "order_status.client_id");
            assert_eq!(order_status.why_held, "", "order_status.why_held");
            assert_eq!(order_status.market_cap_price, 0.0, "order_status.market_cap_price");
        } else {
            assert!(false, "message[1] expected order status notification");
        }

        if let Some(PlaceOrder::ExecutionData(execution_data)) = notifications.next() {
            let contract = execution_data.contract;
            let execution = execution_data.execution;

            assert_eq!(execution_data.request_id, -1, "execution_data.request_id");
            assert_eq!(execution.order_id, 13, "execution.order_id");
            assert_eq!(contract.contract_id, 76792991, "contract.contract_id");
            assert_eq!(contract.symbol, Symbol::from("TSLA"), "contract.symbol");
            assert_eq!(contract.security_type, SecurityType::Stock, "contract.security_type");
            assert_eq!(
                contract.last_trade_date_or_contract_month, "",
                "contract.last_trade_date_or_contract_month"
            );
            assert_eq!(contract.strike, 0.0, "contract.strike");
            assert_eq!(contract.right, "", "contract.right");
            assert_eq!(contract.multiplier, "", "contract.multiplier");
            assert_eq!(contract.exchange, Exchange::from("ISLAND"), "contract.exchange");
            assert_eq!(contract.currency, Currency::from("USD"), "contract.currency");
            assert_eq!(contract.local_symbol, "TSLA", "contract.local_symbol");
            assert_eq!(contract.trading_class, "NMS", "contract.trading_class");
            assert_eq!(execution.execution_id, "00025b46.63f8f39c.01.01", "execution.execution_id");
            assert_eq!(execution.time, "20230224  12:04:56", "execution.time");
            assert_eq!(execution.account_number, "DU1234567", "execution.account_number");
            assert_eq!(execution.exchange, "ISLAND", "execution.exchange");
            assert_eq!(execution.side, "BOT", "execution.side");
            assert_eq!(execution.shares, 100.0, "execution.shares");
            assert_eq!(execution.price, 196.52, "execution.price");
            assert_eq!(execution.perm_id, 1376327563, "execution.perm_id");
            assert_eq!(execution.client_id, 100, "execution.client_id");
            assert_eq!(execution.liquidation, 0, "execution.liquidation");
            assert_eq!(execution.cumulative_quantity, 100.0, "execution.cumulative_quantity");
            assert_eq!(execution.average_price, 196.52, "execution.average_price");
            assert_eq!(execution.order_reference, "", "execution.order_reference");
            assert_eq!(execution.ev_rule, "", "execution.ev_rule");
            assert_eq!(execution.ev_multiplier, None, "execution.ev_multiplier");
            assert_eq!(execution.model_code, "", "execution.model_code");
            assert_eq!(execution.last_liquidity, Liquidity::RemovedLiquidity, "execution.last_liquidity");
        } else {
            assert!(false, "message[2] expected execution notification");
        }

        if let Some(PlaceOrder::OpenOrder(open_order)) = notifications.next() {
            let order_state = &open_order.order_state;

            assert_eq!(open_order.order_id, 13, "open_order.order_id");
            assert_eq!(order_state.status, "Filled", "order_state.status");
        } else {
            assert!(false, "message[3] expected an open order notification");
        }

        if let Some(PlaceOrder::OrderStatus(order_status)) = notifications.next() {
            assert_eq!(order_status.order_id, 13, "order_status.order_id");
            assert_eq!(order_status.status, "Filled", "order_status.status");
            assert_eq!(order_status.filled, 100.0, "order_status.filled");
            assert_eq!(order_status.remaining, 0.0, "order_status.remaining");
            assert_eq!(order_status.average_fill_price, 196.52, "order_status.average_fill_price");
            assert_eq!(order_status.last_fill_price, 196.52, "order_status.last_fill_price");
        } else {
            assert!(false, "message[4] expected order status notification");
        }

        if let Some(PlaceOrder::OpenOrder(open_order)) = notifications.next() {
            let order_state = &open_order.order_state;

            assert_eq!(open_order.order_id, 13, "open_order.order_id");
            assert_eq!(order_state.status, "Filled", "order_state.status");
            assert_eq!(order_state.commission, Some(1.0), "order_state.commission");
            assert_eq!(order_state.minimum_commission, None, "order_state.minimum_commission");
            assert_eq!(order_state.maximum_commission, None, "order_state.maximum_commission");
            assert_eq!(order_state.commission_currency, "USD", "order_state.commission_currency");
        } else {
            assert!(false, "message[5] expected an open order notification");
        }

        if let Some(PlaceOrder::CommissionReport(report)) = notifications.next() {
            assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
            assert_eq!(report.commission, 1.0, "report.commission");
            assert_eq!(report.currency, "USD", "report.currency");
            assert_eq!(report.realized_pnl, None, "report.realized_pnl");
            assert_eq!(report.yields, None, "report.yielded");
            assert_eq!(report.yield_redemption_date, "", "report.yield_redemption_date");
        } else {
            assert!(false, "message[6] expected a commission report notification");
        }
    }

    #[test]
    fn cancel_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "3|41|Cancelled|0|100|0|71270927|0|0|100||0||".to_owned(),
                "4|2|41|202|Order Canceled - reason:||".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let order_id = 41;
        let results = client.cancel_order(order_id, "");

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode(), "4\x001\x0041\x00");

        assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());

        let results = results.unwrap();

        if let Some(CancelOrder::OrderStatus(order_status)) = results.next() {
            assert_eq!(order_status.order_id, 41, "order_status.order_id");
            assert_eq!(order_status.status, "Cancelled", "order_status.status");
            assert_eq!(order_status.filled, 0.0, "order_status.filled");
            assert_eq!(order_status.remaining, 100.0, "order_status.remaining");
            assert_eq!(order_status.average_fill_price, 0.0, "order_status.average_fill_price");
            assert_eq!(order_status.perm_id, 71270927, "order_status.perm_id");
            assert_eq!(order_status.parent_id, 0, "order_status.parent_id");
            assert_eq!(order_status.last_fill_price, 0.0, "order_status.last_fill_price");
            assert_eq!(order_status.client_id, 100, "order_status.client_id");
            assert_eq!(order_status.why_held, "", "order_status.why_held");
            assert_eq!(order_status.market_cap_price, 0.0, "order_status.market_cap_price");
        }

        if let Some(CancelOrder::Notice(notice)) = results.next() {
            assert_eq!(notice.message, "Order Canceled - reason:", "order status notice");
        }
    }

    #[test]
    fn global_cancel() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let results = super::global_cancel(&client);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode(), "58\x001\x00");
        assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
    }

    #[test]
    fn next_valid_order_id() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let results = super::next_valid_order_id(&client);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode(), "8\x001\x000\x00");

        assert!(results.is_ok(), "failed to request next order id: {}", results.err().unwrap());
        assert_eq!(43, results.unwrap(), "next order id");
    }

    #[test]
    fn completed_orders() {
        let _ = env_logger::try_init();

        let message_bus = Arc::new(MessageBusStub::with_responses(vec![
            // Copy exact format from integration test, just changing account to DU1234567
            "101|265598|AAPL|STK||0|||SMART|USD|AAPL|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||1377295418|0|0|0|||||||||||0||-1||||||2147483647|0|0||3|0||0|None||0|0|0||0|0||||0|0|0|2147483647|2147483647||||IB|0|0||0|Filled|100|0|0|150.25|1.7976931348623157E308|0|1|0||0|2147483647|0|Not an insider or substantial shareholder|0|0|9223372036854775807|20231122 10:30:00 America/Los_Angeles|Filled||||||".to_owned(),
            "102|".to_owned(),
        ]));

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let api_only = true;
        let results = super::completed_orders(&client, api_only);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode(), "99\x001\x00");

        assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());

        let results = results.unwrap();
        if let Some(Orders::OrderData(order_data)) = results.next() {
            assert_eq!(order_data.order_id, -1, "open_order.order_id");

            let contract = &order_data.contract;
            let order = &order_data.order;
            let order_state = &order_data.order_state;

            assert_eq!(contract.contract_id, 265598, "contract.contract_id");
            assert_eq!(contract.symbol, Symbol::from("AAPL"), "contract.symbol");
            assert_eq!(contract.security_type, SecurityType::Stock, "contract.security_type");
            assert_eq!(
                contract.last_trade_date_or_contract_month, "",
                "contract.last_trade_date_or_contract_month"
            );
            assert_eq!(contract.strike, 0.0, "contract.strike");
            assert_eq!(contract.right, "", "contract.right");
            assert_eq!(contract.multiplier, "", "contract.multiplier");
            assert_eq!(contract.exchange, Exchange::from("SMART"), "contract.exchange");
            assert_eq!(contract.currency, Currency::from("USD"), "contract.currency");
            assert_eq!(contract.local_symbol, "AAPL", "contract.local_symbol");
            assert_eq!(contract.trading_class, "NMS", "contract.trading_class");
            assert_eq!(order.action, Action::Buy, "order.action");
            assert_eq!(order.total_quantity, 100.0, "order.total_quantity");
            assert_eq!(order.order_type, "MKT", "order.order_type");
            assert_eq!(order.limit_price, Some(0.0), "order.limit_price");
            assert_eq!(order.aux_price, Some(0.0), "order.aux_price");
            assert_eq!(order.tif, TimeInForce::Day, "order.tif");
            assert_eq!(order.oca_group, "", "order.oca_group");
            assert_eq!(order.account, "DU1234567", "order.account");
            assert_eq!(order.open_close, None, "order.open_close");
            assert_eq!(order.origin, OrderOrigin::Customer, "order.origin");
            assert_eq!(order.order_ref, "", "order.order_ref");
            assert_eq!(order.perm_id, 1377295418, "order.perm_id");
            assert_eq!(order.outside_rth, false, "order.outside_rth");
            assert_eq!(order.hidden, false, "order.hidden");
            assert_eq!(order.discretionary_amt, 0.0, "order.discretionary_amt");
            assert_eq!(order.good_after_time, "", "order.good_after_time");
            assert_eq!(order.fa_group, "", "order.fa_group");
            assert_eq!(order.fa_method, "", "order.fa_method");
            assert_eq!(order.fa_percentage, "", "order.fa_percentage");
            assert_eq!(order.fa_profile, "", "order.fa_profile");
            assert_eq!(order.model_code, "", "order.model_code");
            assert_eq!(order.good_till_date, "", "order.good_till_date");
            assert_eq!(order.rule_80_a, None, "order.rule_80_a");
            assert_eq!(order.percent_offset, None, "order.percent_offset");
            assert_eq!(order.settling_firm, "", "order.settling_firm");
            assert_eq!(order.short_sale_slot, ShortSaleSlot::None, "order.short_sale_slot");
            assert_eq!(order.designated_location, "", "order.designated_location");
            assert_eq!(order.exempt_code, -1, "order.exempt_code");
            assert_eq!(order.starting_price, None, "order.starting_price");
            assert_eq!(order.stock_ref_price, None, "order.stock_ref_price");
            assert_eq!(order.delta, None, "order.delta");
            assert_eq!(order.stock_range_lower, None, "order.stock_range_lower");
            assert_eq!(order.stock_range_upper, None, "order.stock_range_upper");
            assert_eq!(order.display_size, None, "order.display_size");
            assert_eq!(order.sweep_to_fill, false, "order.sweep_to_fill");
            assert_eq!(order.all_or_none, false, "order.all_or_none");
            assert_eq!(order.min_qty, None, "order.min_qty");
            assert_eq!(order.oca_type, OcaType::ReduceWithoutBlock, "order.oca_type");
            assert_eq!(order.trigger_method, TriggerMethod::Default, "order.trigger_method");
            assert_eq!(order.volatility, None, "order.volatility");
            assert_eq!(order.volatility_type, None, "order.volatility_type");
            assert_eq!(order.delta_neutral_order_type, "None", "order.delta_neutral_order_type");
            assert_eq!(order.delta_neutral_aux_price, None, "order.delta_neutral_aux_price");
            assert_eq!(order.delta_neutral_con_id, 0, "order.delta_neutral_con_id");
            assert_eq!(order.delta_neutral_short_sale, false, "order.delta_neutral_short_sale");
            assert_eq!(order.delta_neutral_short_sale_slot, 0, "order.delta_neutral_short_sale_slot");
            assert_eq!(order.delta_neutral_designated_location, "", "order.delta_neutral_designated_location");
            assert_eq!(order.continuous_update, false, "order.continuous_update");
            assert_eq!(order.reference_price_type, None, "order.reference_price_type");
            assert_eq!(order.trail_stop_price, Some(150.25), "order.trail_stop_price");
            assert_eq!(order.trailing_percent, None, "order.trailing_percent");
            assert_eq!(contract.combo_legs_description, "", "contract.combo_legs_description");
            assert_eq!(contract.combo_legs.len(), 0, "contract.combo_legs.len()");
            assert_eq!(order.order_combo_legs.len(), 0, "order.order_combo_legs.len()");
            assert_eq!(order.smart_combo_routing_params.len(), 0, "order.smart_combo_routing_params.len()");
            assert_eq!(order.scale_init_level_size, None, "order.scale_init_level_size");
            assert_eq!(order.scale_subs_level_size, None, "order.scale_subs_level_size");
            assert_eq!(order.scale_price_increment, None, "order.scale_price_increment");
            assert_eq!(order.hedge_type, "", "order.hedge_type");
            assert_eq!(order.clearing_account, "", "order.clearing_account");
            assert_eq!(order.clearing_intent, "IB", "order.clearing_intent");
            assert_eq!(order.not_held, false, "order.not_held");
            assert_eq!(contract.delta_neutral_contract, None, "contract.delta_neutral_contract");
            assert_eq!(order.algo_strategy, "", "order.algo_strategy");
            assert_eq!(order.algo_params.len(), 0, "order.algo_params.len()");
            assert_eq!(order.solicited, false, "order.solicited");
            assert_eq!(order_state.status, "Filled", "order_state.status");
            assert_eq!(order.randomize_size, false, "order.randomize_size");
            assert_eq!(order.randomize_price, false, "order.randomize_price");
            assert_eq!(order.conditions.len(), 0, "order.conditions.len()");
            assert_eq!(order.trail_stop_price, Some(150.25), "order.trail_stop_price");
            assert_eq!(order.limit_price_offset, None, "order.limit_price_offset");
            assert_eq!(order.cash_qty, Some(0.0), "order.cash_qty");
            assert_eq!(order.dont_use_auto_price_for_hedge, true, "order.dont_use_auto_price_for_hedge");
            assert_eq!(order.is_oms_container, false, "order.is_oms_container");
            assert_eq!(order.auto_cancel_date, "", "order.auto_cancel_date");
            assert_eq!(order.filled_quantity, 0.0, "order.filled_quantity");
            assert_eq!(order.ref_futures_con_id, None, "order.ref_futures_con_id");
            assert_eq!(order.auto_cancel_parent, false, "order.auto_cancel_parent");
            assert_eq!(order.shareholder, "Not an insider or substantial shareholder", "order.shareholder");
            assert_eq!(order.imbalance_only, false, "order.imbalance_only");
            assert_eq!(order.route_marketable_to_bbo, false, "order.route_marketable_to_bbo");
            assert_eq!(order.parent_perm_id, None, "order.parent_perm_id");
            assert_eq!(
                order_state.completed_time, "20231122 10:30:00 America/Los_Angeles",
                "order_state.completed_time"
            );
            assert_eq!(order_state.completed_status, "Filled", "order_state.completed_status");
        } else {
            assert!(false, "expected order data");
        }
    }

    #[test]
    fn open_orders() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let results = super::open_orders(&client);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "5|1|");

        assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());
    }

    #[test]
    fn all_open_orders() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let results = client.all_open_orders();

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "16|1|");

        assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());
    }

    #[test]
    fn auto_open_orders() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let api_only = true;
        let results = client.auto_open_orders(api_only);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "15|1|1|");

        assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());
    }

    #[test]
    fn executions() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let filter = ExecutionFilter {
            client_id: Some(100),
            account_code: "xyz".to_owned(),
            time: "yyyymmdd hh:mm:ss EST".to_owned(),
            symbol: "TSLA".to_owned(),
            security_type: "STK".to_owned(),
            exchange: "ISLAND".to_owned(),
            side: "BUY".to_owned(),
        };
        let results = client.executions(filter);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(
            request_messages[0].encode_simple(),
            "7|3|9000|100|xyz|yyyymmdd hh:mm:ss EST|TSLA|STK|ISLAND|BUY|"
        );

        assert!(results.is_ok(), "failed to request completed orders: {}", results.err().unwrap());
        // assert_eq!(43, results.unwrap(), "next order id");
    }

    #[test]
    fn encode_limit_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let order_id = 12;
        let contract = Contract {
            security_type: SecurityType::Future,
            exchange: Exchange::from("EUREX"),
            currency: Currency::from("EUR"),
            local_symbol: "FGBL MAR 23".to_owned(),
            last_trade_date_or_contract_month: "202303".to_owned(),
            ..Contract::default()
        };
        let order = order_builder::limit_order(Action::Buy, 10.0, 500.00);

        let results = client.place_order(order_id, &contract, &order);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(
            request_messages[0].encode_simple(),
            "3|12|0||FUT|202303|0|||EUREX||EUR|FGBL MAR 23||||BUY|10|LMT|500||DAY||||0||1|0|0|0|0|0|0|0||0||||||||0||-1|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
        );

        assert!(results.is_ok(), "failed to place order: {}", results.err().unwrap());
    }

    #[test]
    fn encode_combo_market_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let order_id = 12; // get next order id
        let contract = {
            let leg_1 = ComboLeg {
                contract_id: 55928698, //WTI future June 2017
                ratio: 1,
                action: "BUY".to_owned(),
                exchange: "IPE".to_owned(),
                ..ComboLeg::default()
            };

            let leg_2 = ComboLeg {
                contract_id: 55850663, //COIL future June 2017
                ratio: 1,
                action: "SELL".to_owned(),
                exchange: "IPE".to_owned(),
                ..ComboLeg::default()
            };

            Contract {
                symbol: Symbol::from("WTI"), // WTI,COIL spread. Symbol can be defined as first leg symbol ("WTI") or currency ("USD").
                security_type: SecurityType::Spread,
                currency: Currency::from("USD"),
                exchange: Exchange::from("SMART"),
                combo_legs: vec![leg_1, leg_2],
                ..Contract::default()
            }
        };
        let order = order_builder::combo_market_order(Action::Sell, 150.0, true);

        let results = client.place_order(order_id, &contract, &order);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(
            request_messages[0].encode_simple(),
            "3|12|0|WTI|BAG||0|||SMART||USD|||||SELL|150|MKT|||DAY||||0||1|0|0|0|0|0|0|0|2|55928698|1|BUY|IPE|0|0||0|55850663|1|SELL|IPE|0|0||0|0|1|NonGuaranteed|1||0||||||||0||-1|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
        );

        assert!(results.is_ok(), "failed to place order: {}", results.err().unwrap());
    }

    #[test]
    fn submit_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract {
            symbol: Symbol::from("AAPL"),
            security_type: SecurityType::Stock,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Contract::default()
        };

        let order_id = 42;
        let order = order_builder::market_order(Action::Buy, 200.0);

        let result = client.submit_order(order_id, &contract, &order);

        let request_messages = client.message_bus.request_messages();

        assert_eq!(
            request_messages[0].encode().replace('\0', "|"),
            "3|42|0|AAPL|STK||0|||SMART||USD|||||BUY|200|MKT|||DAY||||0||1|0|0|0|0|0|0|0||0||||||||0||-1|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0|0|0|||||||0|||||||||0|0|0|0|||0|"
        );

        assert!(result.is_ok(), "failed to submit order: {}", result.err().unwrap());
    }

    #[test]
    fn order_update_stream() {
        let message_bus = Arc::new(MessageBusStub{
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "5|13|76792991|TSLA|STK||0|?||SMART|USD|TSLA|NMS|BUY|100|MKT|0.0|0.0|DAY||DU1234567||0||100|1376327563|0|0|0||1376327563.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|PreSubmitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||".to_owned(),
                "3|13|PreSubmitted|0|100|0|1376327563|0|0|100||0||".to_owned(),
                "11|-1|13|76792991|TSLA|STK||0.0|||ISLAND|USD|TSLA|NMS|00025b46.63f8f39c.01.01|20230224  12:04:56|DU1234567|ISLAND|BOT|100|196.52|1376327563|100|0|100|196.52|||||2||".to_owned(),
                "59|1|00025b46.63f8f39c.01.01|1.0|USD|1.7976931348623157E308|1.7976931348623157E308|||".to_owned(),
            ]
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let stream = client.order_update_stream();
        assert!(stream.is_ok(), "failed to create order update stream: {}", stream.err().unwrap());

        let notifications = stream.unwrap();

        // First event: OpenOrder
        if let Some(OrderUpdate::OpenOrder(open_order)) = notifications.next() {
            assert_eq!(open_order.order_id, 13, "open_order.order_id");
            assert_eq!(open_order.contract.symbol, Symbol::from("TSLA"), "contract.symbol");
            assert_eq!(open_order.order.action, Action::Buy, "order.action");
            assert_eq!(open_order.order.total_quantity, 100.0, "order.total_quantity");
            assert_eq!(open_order.order_state.status, "PreSubmitted", "order_state.status");
        } else {
            assert!(false, "expected open order notification");
        }

        // Second event: OrderStatus
        if let Some(OrderUpdate::OrderStatus(status)) = notifications.next() {
            assert_eq!(status.order_id, 13, "order_status.order_id");
            assert_eq!(status.status, "PreSubmitted", "order_status.status");
            assert_eq!(status.filled, 0.0, "order_status.filled");
            assert_eq!(status.remaining, 100.0, "order_status.remaining");
        } else {
            assert!(false, "expected order status notification");
        }

        // Third event: ExecutionData
        if let Some(OrderUpdate::ExecutionData(exec_data)) = notifications.next() {
            assert_eq!(exec_data.execution.order_id, 13, "execution.order_id");
            assert_eq!(exec_data.execution.shares, 100.0, "execution.shares");
            assert_eq!(exec_data.execution.price, 196.52, "execution.price");
            assert_eq!(exec_data.execution.side, "BOT", "execution.side");
        } else {
            assert!(false, "expected execution data notification");
        }

        // Fourth event: CommissionReport
        if let Some(OrderUpdate::CommissionReport(report)) = notifications.next() {
            assert_eq!(report.execution_id, "00025b46.63f8f39c.01.01", "report.execution_id");
            assert_eq!(report.commission, 1.0, "report.commission");
            assert_eq!(report.currency, "USD", "report.currency");
        } else {
            assert!(false, "expected commission report notification");
        }
    }

    #[test]
    fn order_update_stream_already_subscribed() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        // Create first subscription
        let stream1 = client.order_update_stream();
        assert!(stream1.is_ok(), "failed to create first order update stream");

        // Try to create second subscription - should fail
        let stream2 = client.order_update_stream();
        assert!(stream2.is_err(), "second order update stream should fail");

        match stream2.err().unwrap() {
            Error::AlreadySubscribed => {}
            other => assert!(false, "expected AlreadySubscribed error, got: {:?}", other),
        }
    }
}
