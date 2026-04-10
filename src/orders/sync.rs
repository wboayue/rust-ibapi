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

impl Client {
    /// Requests all *current* open orders in associated accounts at the current moment.
    /// Open orders are returned once; this function does not initiate a subscription.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.all_open_orders().expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn all_open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_all_open_orders()?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
    ///
    /// # Arguments
    /// * `auto_bind` - if set to true, the newly created orders will be assigned an API order ID and implicitly associated with this client. If set to false, future orders will not be.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 0).expect("connection failed");
    ///
    /// let subscription = client.auto_open_orders(false).expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn auto_open_orders(&self, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_auto_open_orders(auto_bind)?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Cancels an active [Order] placed by the same API client ID.
    ///
    /// # Arguments
    /// * `order_id` - ID of the [Order] to cancel.
    /// * `manual_order_cancel_time` - Optional timestamp to specify the cancellation time. Use an empty string to use the current time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let order_id = 15;
    /// let subscription = client.cancel_order(order_id, "").expect("request failed");
    /// for result in subscription {
    ///    println!("{result:?}");
    /// }
    /// ```
    pub fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        if !manual_order_cancel_time.is_empty() {
            self.check_server_version(
                server_versions::MANUAL_ORDER_TIME,
                "It does not support manual order cancel time attribute",
            )?
        }

        let request = encoders::encode_cancel_order(self.server_version, order_id, manual_order_cancel_time)?;
        let subscription = self.send_order(order_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests completed [Order]s.
    ///
    /// # Arguments
    /// * `api_only` - request only orders placed by the API.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.completed_orders(false).expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn completed_orders(&self, api_only: bool) -> Result<Subscription<Orders>, Error> {
        self.check_server_version(server_versions::COMPLETED_ORDERS, "It does not support completed orders requests.")?;

        let request = encoders::encode_completed_orders(api_only)?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestCompletedOrders, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests current day's (since midnight) executions matching the filter.
    ///
    /// Only the current day's executions can be retrieved.
    /// Along with the [orders::ExecutionData], the [orders::CommissionReport] will also be returned.
    /// When requesting executions, a filter can be specified to receive only a subset of them
    ///
    /// # Arguments
    /// * `filter` - filter criteria used to determine which execution reports are returned
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::orders::ExecutionFilter;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = ExecutionFilter{
    ///    side: "BUY".to_owned(),
    ///    ..ExecutionFilter::default()
    /// };
    ///
    /// let subscription = client.executions(filter).expect("request failed");
    /// for execution_data in &subscription {
    ///    println!("{execution_data:?}")
    /// }
    /// ```
    pub fn executions(&self, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
        let request_id = self.next_request_id();

        let request = encoders::encode_executions(self.server_version, request_id, &filter)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Cancels all open [Order]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// client.global_cancel().expect("request failed");
    /// ```
    pub fn global_cancel(&self) -> Result<(), Error> {
        self.check_server_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel requests.")?;

        let message = encoders::encode_global_cancel(self.server_version)?;
        self.send_message(message)?;

        Ok(())
    }

    /// Gets the next valid order ID from the TWS server.
    ///
    /// Unlike [Self::next_order_id], this function requests the next valid order ID from the TWS server and updates the client's internal order ID sequence.
    /// This can be for ensuring that order IDs are unique across multiple clients.
    ///
    /// Use this method when coordinating order IDs across multiple client instances or when you need to synchronize with the server's order ID sequence at the start of a session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// // Connect to the TWS server at the given address with client ID.
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Request the next valid order ID from the server.
    /// let next_valid_order_id = client.next_valid_order_id().expect("request failed");
    /// println!("next_valid_order_id: {next_valid_order_id}");
    /// ```
    pub fn next_valid_order_id(&self) -> Result<i32, Error> {
        let message = encoders::encode_next_valid_order_id()?;

        let subscription = self.send_shared_request(OutgoingMessages::RequestIds, message)?;

        if let Some(Ok(message)) = subscription.next() {
            let order_id_index = 2;
            let next_order_id = message.peek_int(order_id_index)?;

            self.set_next_order_id(next_order_id);

            Ok(next_order_id)
        } else {
            Err(Error::Simple("no response from server".into()))
        }
    }

    /// Requests all open orders places by this specific API client (identified by the API client id).
    /// For client ID 0, this will bind previous manual TWS orders.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let subscription = client.open_orders().expect("request failed");
    /// for order_data in &subscription {
    ///    println!("{order_data:?}")
    /// }
    /// ```
    pub fn open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_open_orders()?;
        let subscription = self.send_shared_request(OutgoingMessages::RequestOpenOrders, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Places or modifies an [Order].
    ///
    /// Submits an [Order] using [Client] for the given [Contract].
    /// Upon successful submission, the client will start receiving events related to the order's activity via the subscription, including order status updates and execution reports.
    ///
    /// # Arguments
    /// * `order_id` - ID for [Order]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [Order] to submit.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::PlaceOrder;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("MSFT").build();
    /// let order = client.order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .build_order()
    ///     .expect("failed to build order");
    /// let order_id = client.next_order_id();
    ///
    /// let events = client.place_order(order_id, &contract, &order).expect("request failed");
    ///
    /// for event in &events {
    ///     match event {
    ///         PlaceOrder::OrderStatus(order_status) => {
    ///             println!("order status: {order_status:?}")
    ///         }
    ///         PlaceOrder::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///         PlaceOrder::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///         PlaceOrder::CommissionReport(report) => println!("commission report: {report:?}"),
    ///         PlaceOrder::Message(message) => println!("message: {message:?}"),
    ///    }
    /// }
    /// ```
    pub fn place_order(&self, order_id: i32, contract: &Contract, order: &super::Order) -> Result<Subscription<PlaceOrder>, Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(self.server_version, order_id, contract, order)?;
        let subscription = self.send_order(order_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Submits or modifies an [Order] without returning a subscription.
    ///
    /// This is a fire-and-forget method that submits an [Order] for the given [Contract]
    /// but does not return a subscription for order updates. To receive order status updates,
    /// fills, and commission reports, use the [`order_update_stream`](Client::order_update_stream) method
    /// or use [`place_order`](Client::place_order) instead which returns a subscription.
    ///
    /// # Arguments
    /// * `order_id` - ID for [Order]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [Order] to submit.
    ///
    /// # Returns
    /// * `Ok(())` if the order was successfully sent
    /// * `Err(Error)` if validation failed or sending failed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// # fn main() -> Result<(), ibapi::Error> {
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100)?;
    ///
    /// let contract = Contract::stock("MSFT").build();
    /// let order = client.order(&contract)
    ///     .buy(100)
    ///     .market()
    ///     .build_order()?;
    /// let order_id = client.next_order_id();
    ///
    /// // Submit order without waiting for confirmation
    /// client.submit_order(order_id, &contract, &order)?;
    ///
    /// // Monitor all order updates via the order update stream
    /// // This will receive updates for ALL orders, not just this one
    /// use ibapi::orders::OrderUpdate;
    /// for event in client.order_update_stream()? {
    ///     match event {
    ///         OrderUpdate::OrderStatus(status) => println!("Order Status: {status:?}"),
    ///         OrderUpdate::ExecutionData(exec) => println!("Execution: {exec:?}"),
    ///         OrderUpdate::CommissionReport(report) => println!("Commission: {report:?}"),
    ///         _ => {}
    ///     }
    /// }
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn submit_order(&self, order_id: i32, contract: &Contract, order: &super::Order) -> Result<(), Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(self.server_version, order_id, contract, order)?;
        self.send_message(request)?;

        Ok(())
    }

    /// Creates a subscription stream for receiving real-time order updates.
    ///
    /// This method establishes a stream that receives all order-related events including:
    /// - Order status updates (e.g., submitted, filled, cancelled)
    /// - Open order information
    /// - Execution data for trades
    /// - Commission reports
    /// - Order-related messages and notices
    ///
    /// The stream will receive updates for all orders placed through this client connection,
    /// including both new orders submitted after creating the stream and existing orders.
    ///
    /// # Returns
    ///
    /// Returns a `Subscription<OrderUpdate>` that yields `OrderUpdate` enum variants containing:
    /// - `OrderStatus`: Current status of an order (filled amount, average price, etc.)
    /// - `OpenOrder`: Complete order details including contract and order parameters
    /// - `ExecutionData`: Details about individual trade executions
    /// - `CommissionReport`: Commission information for executed trades
    /// - `Message`: Notices or error messages related to orders
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription cannot be created, typically due to
    /// connection issues or internal errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::orders::OrderUpdate;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Create order update stream
    /// let updates = client.order_update_stream().expect("failed to create stream");
    ///
    /// // Process order updates
    /// for update in updates {
    ///     match update {
    ///         OrderUpdate::OrderStatus(status) => {
    ///             println!("Order {} status: {} - filled: {}/{}",
    ///                 status.order_id, status.status, status.filled, status.remaining);
    ///         },
    ///         OrderUpdate::OpenOrder(order_data) => {
    ///             println!("Open order {}: {} {} @ {}",
    ///                 order_data.order.order_id,
    ///                 order_data.order.action,
    ///                 order_data.order.total_quantity,
    ///                 order_data.order.limit_price.unwrap_or(0.0));
    ///         },
    ///         OrderUpdate::ExecutionData(exec) => {
    ///             println!("Execution: {} {} @ {} on {}",
    ///                 exec.execution.side,
    ///                 exec.execution.shares,
    ///                 exec.execution.price,
    ///                 exec.execution.exchange);
    ///         },
    ///         OrderUpdate::CommissionReport(report) => {
    ///             println!("Commission: ${} for execution {}",
    ///                 report.commission, report.execution_id);
    ///         },
    ///         OrderUpdate::Message(notice) => {
    ///             println!("Order message: {}", notice.message);
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This stream provides updates for all orders, not just a specific order.
    /// To track a specific order, filter the updates by order ID.
    pub fn order_update_stream(&self) -> Result<Subscription<OrderUpdate>, Error> {
        let subscription = self.create_order_update_subscription()?;
        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Exercises an options contract.
    ///
    /// Note: this function is affected by a TWS setting which specifies if an exercise request must be finalized.
    ///
    /// # Arguments
    /// * `contract`          - The option [Contract] to be exercised.
    /// * `exercise_action`   - Exercise option. ExerciseAction::Exercise or ExerciseAction::Lapse.
    /// * `exercise_quantity` - Number of contracts to be exercised.
    /// * `account`           - Destination account.
    /// * `ovrd`              - Specifies whether your setting will override the system's natural action. For example, if your action is "exercise" and the option is not in-the-money, by natural action the option would not exercise. If you have override set to true the natural action would be overridden and the out-of-the money option would be exercised.
    /// * `manual_order_time` - Specify the time at which the options should be exercised. If `None`, the current time will be used. Requires TWS API 10.26 or higher.
    pub fn exercise_options(
        &self,
        contract: &Contract,
        exercise_action: ExerciseAction,
        exercise_quantity: i32,
        account: &str,
        ovrd: bool,
        manual_order_time: Option<OffsetDateTime>,
    ) -> Result<Subscription<ExerciseOptions>, Error> {
        let order_id = self.next_order_id();

        let request = encoders::encode_exercise_options(
            self.server_version,
            order_id,
            contract,
            exercise_action,
            exercise_quantity,
            account,
            ovrd,
            manual_order_time,
        )?;
        let subscription = self.send_order(order_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }
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

        let results = client.global_cancel();

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode(), "58\x001\x00");
        assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
    }

    #[test]
    fn cancel_order_cme_tagging() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["3|41|Cancelled|0|100|0|71270927|0|0|100||0||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::CME_TAGGING_FIELDS);

        let order_id = 41;
        let results = client.cancel_order(order_id, "");

        let request_messages = client.message_bus.request_messages();

        // No VERSION field, has empty ext_operator and i32::MAX manual_order_indicator
        assert_eq!(request_messages[0].encode(), format!("4\x0041\x00\x00\x00{}\x00", i32::MAX));

        assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
    }

    #[test]
    fn global_cancel_cme_tagging() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, server_versions::CME_TAGGING_FIELDS);

        let results = client.global_cancel();

        let request_messages = client.message_bus.request_messages();

        // No VERSION field, has empty ext_operator and i32::MAX manual_order_indicator
        assert_eq!(request_messages[0].encode(), format!("58\x00\x00{}\x00", i32::MAX));
        assert!(results.is_ok(), "failed to cancel order: {}", results.err().unwrap());
    }

    #[test]
    fn next_valid_order_id() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["9|1|43||".to_owned()],
        });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let results = client.next_valid_order_id();

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
        let results = client.completed_orders(api_only);

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

        let results = client.open_orders();

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
            ..Default::default()
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
