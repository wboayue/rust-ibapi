//! Asynchronous implementation of order management functionality

use crate::common::request_helpers::{self, expect_proto};
use crate::messages::OutgoingMessages;
use crate::orders::OrderId;
use crate::subscriptions::{Subscription, SubscriptionItem};
use crate::{Client, Error};
use futures::{FutureExt, StreamExt};

use std::time::{Duration, Instant};

use super::common::fill::{completed_status, lists_order, lookup_wait, FillTracker};
use super::common::{decoders, encoders, verify};
use super::*;

impl Client {
    /// Start building an order for the given contract
    ///
    /// This is the primary API for creating orders, providing a fluent interface
    /// that guides you through the order creation process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let order_id = client.order(&contract)
    ///         .buy(100)
    ///         .limit(50.0)
    ///         .submit().await.expect("order submission failed");
    /// }
    /// ```
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<ClientBound<'a, Self>> {
        OrderBuilder::new(self, contract)
    }

    /// Subscribes to order update events. Only one subscription can be active at a time.
    ///
    /// Order-bound TWS errors and warnings (e.g. rejections, code 399 order
    /// messages) arrive as [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem),
    /// not as [`OrderUpdate`] variants. They surface via `next()` as below;
    /// `filter_data()` drops them (logging at `warn!` level), so match on
    /// notices explicitly when monitoring fire-and-forget orders for rejection.
    ///
    /// To pair a [`CommissionReport`] with the
    /// [`ExecutionData`] it belongs to, join on
    /// `execution_id` — the commission follows its execution and shares that key. See
    /// the [`CommissionReport`] docs for the idiom.
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
    /// After an error, drop the ended subscription (and any clones) and call
    /// `order_update_stream` again; while one is held, a second call
    /// returns [`Error::AlreadySubscribed`].
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let mut stream = client.order_update_stream().await.expect("failed to create stream");
    ///     while let Some(item) = stream.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(OrderUpdate::OrderStatus(s))) => println!("status: {s:?}"),
    ///             Ok(SubscriptionItem::Data(update)) => println!("update: {update:?}"),
    ///             Ok(SubscriptionItem::Notice(notice)) if notice.is_error() => {
    ///                 eprintln!("order {:?} rejected: {}", notice.request_id, notice.message);
    ///             }
    ///             Ok(SubscriptionItem::Notice(notice)) => println!("notice: {}", notice.message),
    ///             Err(e) => { eprintln!("err: {e:?}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn order_update_stream(&self) -> Result<Subscription<OrderUpdate>, Error> {
        let internal_subscription = self.create_order_update_subscription().await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
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
    /// use futures::StreamExt;
    /// use ibapi::orders::OrderUpdate;
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///
    ///     let contract = Contract::stock("MSFT").build();
    ///     let order = client.order(&contract).buy(100).market().build_order()?;
    ///     let order_id = client.next_order_id();
    ///
    ///     // Submit order without waiting for confirmation
    ///     client.submit_order(order_id, &contract, &order).await?;
    ///
    ///     // Monitor all order updates via the order update stream
    ///     // This will receive updates for ALL orders, not just this one
    ///     let mut events = client.order_update_stream().await?.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         match event? {
    ///             OrderUpdate::OrderStatus(status) => println!("Order Status: {status:?}"),
    ///             OrderUpdate::ExecutionData(exec) => println!("Execution: {exec:?}"),
    ///             OrderUpdate::CommissionReport(report) => println!("Commission: {report:?}"),
    ///             OrderUpdate::OrderBound(binding) => println!("Order binding: {binding:?}"),
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn submit_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &Order) -> Result<(), Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self.server_version(), order)?;

        let request = encoders::encode_place_order(checked_id.value(), contract, order)?;
        self.send_message(request).await?;

        Ok(())
    }

    /// Submit multiple OCA (One-Cancels-All) orders
    ///
    /// When one order in the group is filled, all others are automatically cancelled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::OcaType;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     
    ///     let contract1 = Contract::stock("AAPL").build();
    ///     let contract2 = Contract::stock("MSFT").build();
    ///
    ///     let order1 = client.order(&contract1)
    ///         .buy(100)
    ///         .limit(50.0)
    ///         .oca_group("MyOCA", OcaType::CancelWithBlock)
    ///         .build_order().expect("order build failed");
    ///         
    ///     let order2 = client.order(&contract2)
    ///         .buy(100)
    ///         .limit(45.0)
    ///         .oca_group("MyOCA", OcaType::CancelWithBlock)
    ///         .build_order().expect("order build failed");
    ///
    ///     let order_ids = client.submit_oca_orders(
    ///         vec![(contract1, order1), (contract2, order2)]
    ///     ).await.expect("OCA submission failed");
    /// }
    /// ```
    pub async fn submit_oca_orders(&self, orders: Vec<(crate::contracts::Contract, crate::orders::Order)>) -> Result<Vec<OrderId>, Error> {
        let mut order_ids = Vec::new();

        for (contract, mut order) in orders.into_iter() {
            let order_id = self.next_order_id();
            order.order_id = order_id;
            order_ids.push(OrderId::new(order_id));
            self.submit_order(order_id, &contract, &order).await?;
        }

        Ok(order_ids)
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
    /// use futures::StreamExt;
    /// use ibapi::orders::PlaceOrder;
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ibapi::Error> {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
    ///
    ///     let contract = Contract::stock("MSFT").build();
    ///     let order = client.order(&contract).buy(100).market().build_order()?;
    ///     let order_id = client.next_order_id();
    ///
    ///     let mut events = client.place_order(order_id, &contract, &order).await?.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         match event? {
    ///             PlaceOrder::OrderStatus(order_status) => println!("order status: {order_status:?}"),
    ///             PlaceOrder::OpenOrder(open_order) => println!("open order: {open_order:?}"),
    ///             PlaceOrder::ExecutionData(execution) => println!("execution: {execution:?}"),
    ///             PlaceOrder::CommissionReport(report) => println!("commission report: {report:?}"),
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn place_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self.server_version(), order)?;

        let request = encoders::encode_place_order(checked_id.value(), contract, order)?;
        let internal_subscription = self.send_order(checked_id, request).await?;

        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Cancels an open [Order].
    ///
    /// The confirmation (TWS code 202) arrives as a non-terminal
    /// [`SubscriptionItem::Notice`](crate::subscriptions::SubscriptionItem);
    /// the subscription stays open until dropped, so break once cancellation
    /// is observed.
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id` is at or above 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     // `""` selects immediate cancel (no manual order time).
    ///     let mut subscription = client.cancel_order(42, "").await.expect("cancel failed");
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(event)) => println!("status: {event:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) if n.is_cancellation() => {
    ///                 println!("cancelled: {n}");
    ///                 break;
    ///             }
    ///             Ok(SubscriptionItem::Notice(n)) => println!("notice: {n}"),
    ///             Err(e) => { eprintln!("cancel err: {e:?}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn cancel_order(&self, order_id: impl Into<OrderId>, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        let order_id = order_id.into().checked()?;
        let request = encoders::encode_cancel_order(order_id.value(), manual_order_cancel_time)?;
        let internal_subscription = self.send_order(order_id, request).await?;

        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Cancels all open [Order]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     client.global_cancel().await.expect("global_cancel failed");
    /// }
    /// ```
    pub async fn global_cancel(&self) -> Result<(), Error> {
        let message = encoders::encode_global_cancel()?;
        self.send_message(message).await?;

        Ok(())
    }

    /// Gets next valid order id
    ///
    /// The returned value also raises the client's order-ID generator to at
    /// least that value — monotonically, never lowering it below locally
    /// allocated order IDs, including IDs whose order has not yet reached the
    /// server.
    ///
    /// # Errors
    /// - [`Error::OrderIdInRequestRange`] if TWS returns an id at or above 1,500,000,000 (reserved for request ids).
    /// - [`Error::Timeout`] if TWS sends no answer within 30 seconds, or within [`one_shot_timeout`](crate::ClientBuilder::one_shot_timeout) when set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let next = client.next_valid_order_id().await.expect("next id failed");
    ///     println!("next_valid_order_id: {next}");
    /// }
    /// ```
    pub async fn next_valid_order_id(&self) -> Result<i32, Error> {
        let next_order_id = request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestIds,
            encoders::encode_next_valid_order_id,
            expect_proto(decoders::decode_next_valid_id_proto),
        )
        .await?;

        let next_order_id = OrderId::from(next_order_id).checked()?;
        self.raise_next_order_id(next_order_id);
        Ok(next_order_id.value())
    }

    /// Requests completed [Order]s.
    ///
    /// A completed order's `total_quantity` is the quantity left unfilled, not the size
    /// ordered: `0` for a filled order. The size ordered is `total_quantity + filled_quantity`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.completed_orders(true).await.expect("completed_orders failed");
    ///     let mut orders = subscription.filter_data();
    ///     while let Some(order) = orders.next().await {
    ///         println!("{order:?}");
    ///     }
    /// }
    /// ```
    pub async fn completed_orders(&self, api_only: bool) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_completed_orders(api_only)?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestCompletedOrders, request).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Requests all open orders placed by this specific API client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.open_orders().await.expect("open_orders failed");
    ///     let mut orders = subscription.filter_data();
    ///     while let Some(order) = orders.next().await {
    ///         println!("{order:?}");
    ///     }
    /// }
    /// ```
    pub async fn open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_open_orders()?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Requests all *current* open orders in associated accounts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let subscription = client.all_open_orders().await.expect("all_open_orders failed");
    ///     let mut orders = subscription.filter_data();
    ///     while let Some(order) = orders.next().await {
    ///         println!("{order:?}");
    ///     }
    /// }
    /// ```
    pub async fn all_open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_all_open_orders()?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Requests status updates about future orders placed from TWS. Can only be used with client ID 0.
    ///
    /// # Arguments
    /// * `auto_bind` - if set to true, the newly created orders will be assigned an API order ID and implicitly associated with this client. If set to false, future orders will not be.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 0).await.expect("connection failed");
    ///     let subscription = client.auto_open_orders(true).await.expect("auto_open_orders failed");
    ///     let mut orders = subscription.filter_data();
    ///     while let Some(order) = orders.next().await {
    ///         println!("{order:?}");
    ///     }
    /// }
    /// ```
    pub async fn auto_open_orders(&self, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_auto_open_orders(auto_bind)?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Requests executions matching the filter.
    ///
    /// Covers the current day (since midnight) by default; set
    /// [`ExecutionFilter::last_n_days`] or [`ExecutionFilter::specific_dates`]
    /// to reach earlier days.
    ///
    /// Both [`ExecutionData`] and
    /// [`CommissionReport`] are delivered on this
    /// stream. Join a commission to its execution by `execution_id`
    /// (see the [`CommissionReport`] docs) — the commission follows its execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::orders::{ExecutionFilter, ExecutionFilterSide};
    /// use ibapi::subscriptions::SubscriptionItem;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let filter = ExecutionFilter {
    ///         side: Some(ExecutionFilterSide::Buy),
    ///         ..ExecutionFilter::default()
    ///     };
    ///     let mut subscription = client.executions(filter).await.expect("request failed");
    ///
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(ex))  => println!("{ex:?}"),
    ///             Ok(SubscriptionItem::Notice(n)) => eprintln!("notice: {n}"),
    ///             Err(e) => eprintln!("Error: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn executions(&self, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
        let request_id = self.mint_request_id();
        let request = encoders::encode_executions(request_id.raw(), &filter)?;
        let internal_subscription = self.message_bus.send_executions_request(request_id, request).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Streams one order's status updates.
    ///
    /// Starts with the order's latest status, if this client has seen one
    /// (the latest statuses of the 10,000 most recent orders are kept), then
    /// yields each new [`OrderStatus`] as TWS sends it. A reconnect keeps only
    /// statuses of orders that ended: TWS does not replay what happened while
    /// disconnected, so a working order's stream starts empty until its next
    /// status. If TWS refuses the
    /// order with an error before any status (e.g. 110, a price off the
    /// minimum tick), the stream yields that error as [`Error::Notice`] and
    /// ends; a stream opened later starts with it. Otherwise the stream does
    /// not end on its own when the order does; stop reading, or see
    /// [`wait_for_fill`](Self::wait_for_fill) for the common case.
    ///
    /// Takes nothing from [`order_update_stream`](Self::order_update_stream) or
    /// a [`place_order`](Self::place_order) subscription: all may be open at
    /// once, as may several streams on one order. Nothing is sent to TWS.
    ///
    /// # Errors
    ///
    /// [`Error::OrderIdInRequestRange`] if `order_id` is at or above 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::StreamExt;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::SubscriptionItemStreamExt;
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let order_id = client.order(&contract).buy(100).limit(150.0).submit().await.expect("order submission failed");
    ///
    ///     let mut statuses = client.order_status_stream(order_id).await.expect("stream failed").filter_data();
    ///     while let Some(status) = statuses.next().await {
    ///         let status = status.expect("stream error");
    ///         println!("{}: {}/{} filled", status.status, status.filled, status.filled + status.remaining);
    ///         if status.status.is_terminal() {
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn order_status_stream(&self, order_id: impl Into<OrderId>) -> Result<Subscription<OrderStatus>, Error> {
        let internal_subscription = self.message_bus.create_order_status_subscription(order_id.into().checked()?).await?;
        Ok(Subscription::new_from_internal_simple(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Waits until an order fills or ends, or `timeout` elapses.
    ///
    /// Returns [`OrderOutcome::Filled`] when the order filled completely,
    /// [`OrderOutcome::Ended`] when it was cancelled or became inactive first,
    /// [`OrderOutcome::Rejected`] with TWS's error when TWS refused the order
    /// without a status (e.g. 110, a price off the minimum tick), and
    /// [`OrderOutcome::TimedOut`] with the last status seen when time runs
    /// out. A timeout leaves the order working; cancel it yourself if needed.
    ///
    /// Built on [`order_status_stream`](Self::order_status_stream), so an
    /// order that already finished before the call is reported at once, as
    /// long as this client saw its status since connecting. When it has seen
    /// none, the wait first looks the order up: an order this client's open
    /// orders ([`open_orders`](Self::open_orders)) list is still working, and
    /// the wait goes on; otherwise one found among
    /// [`completed_orders`](Self::completed_orders) ends the wait at once,
    /// with a status built from the completed order, whose fill prices are
    /// `None` (see [`executions`](Self::executions) for them). Each lookup
    /// reads its replies to the end, which can add a round trip to `timeout`.
    /// With client id 0, the open-orders request also binds manual TWS orders
    /// to this client. An order placed by another client, or one in neither
    /// reply, waits out the timeout.
    ///
    /// # Errors
    ///
    /// [`Error::OrderIdInRequestRange`] if `order_id` is at or above 1,500,000,000
    /// (reserved for request ids), [`Error::ConnectionReset`] if the connection is
    /// down or drops during the wait, [`Error::Shutdown`] if the client shuts down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use ibapi::contracts::Contract;
    /// use ibapi::orders::OrderOutcome;
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///
    ///     let order_id = client.order(&contract).buy(100).market().submit().await.expect("order submission failed");
    ///
    ///     match client.wait_for_fill(order_id, Duration::from_secs(30)).await.expect("wait failed") {
    ///         OrderOutcome::Filled(status) => println!("filled {} @ {:?}", status.filled, status.average_fill_price),
    ///         OrderOutcome::Ended(status) => println!("ended {:?} after {} filled", status.status, status.filled),
    ///         OrderOutcome::TimedOut(status) => println!("still working: {status:?}"),
    ///         OrderOutcome::Rejected(notice) => println!("rejected: {notice}"),
    ///     }
    /// }
    /// ```
    pub async fn wait_for_fill(&self, order_id: impl Into<OrderId>, timeout: Duration) -> Result<OrderOutcome, Error> {
        let order_id = order_id.into();
        let mut statuses = self.order_status_stream(order_id).await?;
        let mut tracker = FillTracker::default();
        let deadline = Instant::now().checked_add(timeout);
        // Nothing ready: this client has no status for the order (e.g.
        // placed on an earlier connection). Look it up.
        let first = statuses.next().now_or_never();
        if first.is_none() {
            if let Some(status) = self.find_finished(order_id, deadline).await? {
                if let Some(outcome) = tracker.observe(status) {
                    return Ok(outcome);
                }
            }
        }

        let wait = async {
            let mut items = futures::stream::iter(first.flatten()).chain(statuses);
            while let Some(item) = items.next().await {
                match item {
                    Ok(SubscriptionItem::Data(status)) => {
                        if let Some(outcome) = tracker.observe(status) {
                            return Ok(outcome);
                        }
                    }
                    Ok(SubscriptionItem::Notice(_)) => {}
                    Err(error) => return FillTracker::error(error),
                }
            }
            Err(Error::UnexpectedEndOfStream)
        };
        let remaining = deadline.map_or(timeout, |deadline| deadline.saturating_duration_since(Instant::now()));
        match tokio::time::timeout(remaining, wait).await {
            Ok(result) => result,
            Err(_) => Ok(tracker.timed_out()),
        }
    }

    /// Look up an order this client has no status for: `None` when this
    /// client's open orders list it (their statuses reach the order's status
    /// streams like any other) or neither reply does; otherwise its status
    /// from completed orders. Each reply is read to its end marker.
    async fn find_finished(&self, order_id: OrderId, deadline: Option<Instant>) -> Result<Option<OrderStatus>, Error> {
        async fn read(mut subscription: Subscription<Orders>, deadline: Option<Instant>) -> Result<Vec<Orders>, Error> {
            match deadline {
                Some(deadline) => subscription.collect_to_end_within(lookup_wait(deadline)).await,
                None => subscription.collect_to_end().await,
            }
        }
        // A reply cut short (deadline, dropped connection) proves nothing:
        // keep waiting; the status stream reports a dropped connection.
        let Ok(open) = read(self.open_orders().await?, deadline).await else {
            return Ok(None);
        };
        if lists_order(&open, order_id) {
            return Ok(None);
        }
        let Ok(completed) = read(self.completed_orders(false).await?, deadline).await else {
            return Ok(None);
        };
        Ok(completed_status(&completed, self.client_id(), order_id))
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
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::option("AAPL", "20251219", 150.0, OptionRight::Call);
    ///     let subscription = client
    ///         .exercise_options(&contract)
    ///         .exercise(1)
    ///         .override_natural_action()
    ///         .submit()
    ///         .await
    ///         .expect("exercise_options failed");
    /// }
    /// ```
    pub fn exercise_options<'a>(&'a self, contract: &'a Contract) -> ExerciseOptionsBuilder<'a, Self> {
        ExerciseOptionsBuilder::new(self, contract)
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
