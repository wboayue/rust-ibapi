use std::sync::Arc;

use super::common::{decoders, encoders, verify};
use super::{CancelOrder, ClientBound, ExecutionFilter, Executions, ExerciseOptionsBuilder, OrderBuilder, OrderUpdate, Orders, PlaceOrder};
use crate::client::blocking::Subscription;
use crate::common::request_helpers::{self, expect_proto};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::OrderId;
use crate::{client::sync::Client, server_versions, Error};

impl Client {
    /// Start building an order for the given contract
    ///
    /// This is the primary API for creating orders, providing a fluent interface
    /// that guides you through the order creation process.
    ///
    /// # Examples
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    ///
    /// let order_id = client.order(&contract)
    ///     .buy(100)
    ///     .limit(50.0)
    ///     .submit().expect("order submission failed");
    /// ```
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<ClientBound<'a, Self>> {
        OrderBuilder::new(self, contract)
    }

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

    /// Cancels an active [`crate::orders::Order`] placed by the same API client ID.
    ///
    /// The confirmation (TWS code 202) arrives as a non-terminal
    /// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem);
    /// the subscription stays open until dropped, so break once cancellation
    /// is observed.
    ///
    /// # Arguments
    /// * `order_id` - ID of the [`crate::orders::Order`] to cancel.
    /// * `manual_order_cancel_time` - Optional timestamp to specify the cancellation time. Use an empty string to use the current time.
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id` is at or above 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let order_id = 15;
    /// let subscription = client.cancel_order(order_id, "").expect("request failed");
    /// for item in &subscription {
    ///     match item {
    ///         Ok(SubscriptionItem::Data(status)) => println!("status: {status:?}"),
    ///         Ok(SubscriptionItem::Notice(n)) if n.is_cancellation() => {
    ///             println!("cancelled: {n}");
    ///             break;
    ///         }
    ///         Ok(SubscriptionItem::Notice(n)) => println!("notice: {n}"),
    ///         Err(e) => { eprintln!("cancel err: {e:?}"); break; }
    ///     }
    /// }
    /// ```
    pub fn cancel_order(&self, order_id: impl Into<OrderId>, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        if !manual_order_cancel_time.is_empty() {
            self.check_server_version(
                server_versions::MANUAL_ORDER_TIME,
                "It does not support manual order cancel time attribute",
            )?
        }

        let order_id = order_id.into().checked()?;
        let request = encoders::encode_cancel_order(order_id.value(), manual_order_cancel_time)?;
        let subscription = self.send_order(order_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Requests completed [`crate::orders::Order`]s.
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

    /// Requests executions matching the filter.
    ///
    /// Covers the current day (since midnight) by default; set
    /// [`ExecutionFilter::last_n_days`] or [`ExecutionFilter::specific_dates`]
    /// to reach earlier days.
    /// Along with the [`crate::orders::ExecutionData`], the [`crate::orders::CommissionReport`] will also be returned.
    /// Join a commission to its execution by `execution_id` (see the
    /// [`CommissionReport`](crate::orders::CommissionReport) docs) — the commission follows its execution.
    /// When requesting executions, a filter can be specified to receive only a subset of them
    ///
    /// # Arguments
    /// * `filter` - filter criteria used to determine which execution reports are returned
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let filter = ExecutionFilter {
    ///    side: Some(ExecutionFilterSide::Buy),
    ///    ..ExecutionFilter::default()
    /// };
    ///
    /// let subscription = client.executions(filter).expect("request failed");
    /// for execution_data in &subscription {
    ///    println!("{execution_data:?}")
    /// }
    /// ```
    pub fn executions(&self, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
        let request_id = self.mint_request_id();

        let request = encoders::encode_executions(request_id.raw(), &filter)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Cancels all open [`crate::orders::Order`]s.
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
    /// Unlike [Self::next_order_id], this function requests the next valid order ID from the TWS server.
    /// This can be for ensuring that order IDs are unique across multiple clients.
    ///
    /// The returned value also raises the client's order-ID generator to at
    /// least that value — monotonically, never lowering it below locally
    /// allocated order IDs, including IDs whose order has not yet reached the
    /// server.
    ///
    /// Use this method when coordinating order IDs across multiple client instances or when you need to synchronize with the server's order ID sequence at the start of a session.
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if TWS returns an id at or above 1,500,000,000 (reserved for request ids).
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
        let next_order_id = request_helpers::blocking::one_shot_shared(
            self,
            OutgoingMessages::RequestIds,
            encoders::encode_next_valid_order_id,
            expect_proto(decoders::decode_next_valid_id_proto),
        )?;

        let next_order_id = OrderId::from(next_order_id).checked()?;
        self.raise_next_order_id(next_order_id);
        Ok(next_order_id.value())
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

    /// Places or modifies an [`crate::orders::Order`].
    ///
    /// Submits an [`crate::orders::Order`] using [Client] for the given [Contract].
    /// Upon successful submission, the client will start receiving events related to the order's activity via the subscription, including order status updates and execution reports.
    ///
    /// # Arguments
    /// * `order_id` - ID for [`crate::orders::Order`]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [`crate::orders::Order`] to submit.
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id`, or a `parent_id` or preset attached-order id on `order`, is at or above 1,500,000,000 (reserved for request ids).
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
    ///    }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    pub fn place_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &super::Order) -> Result<Subscription<PlaceOrder>, Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self, order, checked_id.value())?;
        verify::verify_order_contract(self, contract, checked_id.value())?;

        let request = encoders::encode_place_order(checked_id.value(), contract, order)?;
        let subscription = self.send_order(checked_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Submits or modifies an [`crate::orders::Order`] without returning a subscription.
    ///
    /// This is a fire-and-forget method that submits an [`crate::orders::Order`] for the given [Contract]
    /// but does not return a subscription for order updates. To receive order status updates,
    /// fills, and commission reports, use the [`order_update_stream`](Client::order_update_stream) method
    /// or use [`place_order`](Client::place_order) instead which returns a subscription.
    ///
    /// # Arguments
    /// * `order_id` - ID for [`crate::orders::Order`]. Get next valid ID using [Client::next_order_id].
    /// * `contract` - [Contract] to submit order for.
    /// * `order` - [`crate::orders::Order`] to submit.
    ///
    /// # Returns
    /// * `Ok(())` if the order was successfully sent
    /// * `Err(Error)` if validation failed or sending failed
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id`, or a `parent_id` or preset attached-order id on `order`, is at or above 1,500,000,000 (reserved for request ids).
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
    ///         OrderUpdate::OrderBound(binding) => println!("Order binding: {binding:?}"),
    ///         _ => {}
    ///     }
    /// }
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn submit_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &super::Order) -> Result<(), Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self, order, checked_id.value())?;
        verify::verify_order_contract(self, contract, checked_id.value())?;

        let request = encoders::encode_place_order(checked_id.value(), contract, order)?;
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
    ///
    /// Order-bound TWS errors and warnings (e.g. rejections, code 399 order
    /// messages) arrive as [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem),
    /// not as `OrderUpdate` variants. They surface via `iter()` / `next()` as
    /// below; `iter_data()` drops them (logging at `warn!` level), so match on
    /// notices explicitly when monitoring fire-and-forget orders for rejection.
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
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Create order update stream
    /// let updates = client.order_update_stream().expect("failed to create stream");
    ///
    /// // `iter()` surfaces both data and notices; `iter_data()` would drop notices.
    /// for item in updates.iter() {
    ///     match item? {
    ///         SubscriptionItem::Data(OrderUpdate::OrderStatus(status)) => {
    ///             println!("Order {} status: {} - filled: {}/{}",
    ///                 status.order_id, status.status, status.filled, status.remaining);
    ///         }
    ///         SubscriptionItem::Data(update) => println!("update: {update:?}"),
    ///         SubscriptionItem::Notice(notice) if notice.is_error() => {
    ///             eprintln!("order {:?} rejected: {}", notice.request_id, notice.message);
    ///         }
    ///         SubscriptionItem::Notice(notice) => println!("notice: {}", notice.message),
    ///     }
    /// }
    /// # Ok::<(), ibapi::Error>(())
    /// ```
    ///
    /// # Note
    ///
    /// This stream provides updates for all orders, not just a specific order.
    /// To track a specific order, filter the updates by order ID.
    ///
    /// To pair a [`CommissionReport`](crate::orders::CommissionReport) with the
    /// [`ExecutionData`](crate::orders::ExecutionData) it belongs to, join on
    /// `execution_id` — the commission follows its execution and shares that key. See
    /// the [`CommissionReport`](crate::orders::CommissionReport) docs for the idiom.
    ///
    /// # Gaps and recovery
    ///
    /// TWS does not replay order events, so two situations leave a gap the
    /// stream cannot fill on its own:
    ///
    /// - **Reconnect.** The stream survives the client's automatic reconnects:
    ///   the same subscription keeps delivering once the connection returns,
    ///   but updates TWS emitted during the outage are lost and no marker
    ///   appears in the stream. Watch [`Self::notice_stream`] for the
    ///   connectivity notices (codes 1100 connectivity lost, 1101 restored
    ///   with data lost, 1102 restored with data maintained, 1300 socket reset).
    /// - **Error.** An `Err` item ends the stream; every later call returns
    ///   `None`. A frame that fails to decode is a bug in TWS or this crate,
    ///   so it is surfaced rather than skipped.
    ///
    /// Client shutdown (disconnect, drop, or a reconnect that gives up) also
    /// ends the stream; after it, `order_update_stream` returns
    /// [`Error::Shutdown`].
    ///
    /// After an error, drop the ended subscription and call
    /// `order_update_stream` again; while the old one is held and not
    /// cancelled, a second call returns [`Error::AlreadySubscribed`].
    /// After a reconnect, the existing subscription is still live. In both
    /// cases, rebuild state from snapshots:
    ///
    /// 1. [`Self::all_open_orders`]: every open order and its status.
    /// 2. [`Self::completed_orders`]`(false)`: orders that filled or were
    ///    cancelled during the gap.
    /// 3. [`Self::executions`] with a default [`ExecutionFilter`]: fills and
    ///    their commissions. The default covers the current day only; set
    ///    `last_n_days` when the gap may span midnight, as the daily gateway
    ///    reset does.
    ///
    /// The snapshots cover every client's orders, while this stream reports
    /// only this client's. Keep the entries whose `order.client_id` /
    /// `execution.client_id` matches, or state will hold orders the stream
    /// never updates.
    ///
    /// Take the snapshots after the stream is live again, so no event falls
    /// between the two. An event can then arrive both ways; apply updates
    /// idempotently, keyed by `perm_id` for orders and `execution_id` for fills.
    pub fn order_update_stream(&self) -> Result<Subscription<OrderUpdate>, Error> {
        let subscription = self.create_order_update_subscription()?;
        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }

    /// Exercise or lapse an option position.
    ///
    /// Terminal: [`ExerciseOptionsBuilder::submit`]. Pick [`exercise`](ExerciseOptionsBuilder::exercise)
    /// or [`lapse`](ExerciseOptionsBuilder::lapse); [`account`](ExerciseOptionsBuilder::account),
    /// [`override_natural_action`](ExerciseOptionsBuilder::override_natural_action) and
    /// [`manual_order_time`](ExerciseOptionsBuilder::manual_order_time) are optional.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::{Contract, OptionRight};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::option("AAPL", "20251219", 150.0, OptionRight::Call);
    /// let subscription = client
    ///     .exercise_options(&contract)
    ///     .exercise(1)
    ///     .override_natural_action()
    ///     .submit()
    ///     .expect("exercise_options failed");
    /// ```
    pub fn exercise_options<'a>(&'a self, contract: &'a Contract) -> ExerciseOptionsBuilder<'a, Self> {
        ExerciseOptionsBuilder::new(self, contract)
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
