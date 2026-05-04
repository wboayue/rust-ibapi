use std::sync::Arc;

use super::common::{encoders, verify};
use super::{CancelOrder, ExecutionFilter, Executions, ExerciseAction, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
use crate::client::blocking::Subscription;
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::{client::sync::Client, server_versions, Error};
use time::OffsetDateTime;

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

        let request = encoders::encode_cancel_order(order_id, manual_order_cancel_time)?;
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

        let request = encoders::encode_executions(request_id, &filter)?;
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

        let message = encoders::encode_global_cancel()?;
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
    /// for event in events.iter_data() {
    ///     match event? {
    ///         PlaceOrder::OrderStatus(order_status) => {
    ///             println!("order status: {order_status:?}")
    ///         }
    ///         PlaceOrder::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///         PlaceOrder::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///         PlaceOrder::CommissionReport(report) => println!("commission report: {report:?}"),
    ///         PlaceOrder::Message(message) => println!("message: {message:?}"),
    ///    }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn place_order(&self, order_id: i32, contract: &Contract, order: &super::Order) -> Result<Subscription<PlaceOrder>, Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(order_id, contract, order)?;
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
    /// for event in client.order_update_stream()?.iter_data() {
    ///     match event? {
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

        let request = encoders::encode_place_order(order_id, contract, order)?;
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
    /// for update in updates.iter_data() {
    ///     match update? {
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
    /// # Ok::<(), ibapi::Error>(())
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

        let request = encoders::encode_exercise_options(order_id, contract, exercise_action, exercise_quantity, account, ovrd, manual_order_time)?;
        let subscription = self.send_order(order_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }
}

#[cfg(test)]
mod tests;
