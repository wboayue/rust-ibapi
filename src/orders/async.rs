//! Asynchronous implementation of order management functionality

use crate::common::request_helpers::{self, expect_proto};
use crate::messages::OutgoingMessages;
use crate::orders::OrderId;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::{decoders, encoders, verify};
use super::*;

impl Client {
    /// Start building an order for the given contract
    ///
    /// This is the primary API for creating orders, providing a fluent interface
    /// that guides you through the order creation process.
    ///
    /// # Examples
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

    /// Submits an Order (fire-and-forget).
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id`, or a `parent_id` or preset attached-order id on `order`, is at or above 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let order = client
    ///         .order(&contract)
    ///         .buy(100)
    ///         .market()
    ///         .build()
    ///         .expect("order build");
    ///     let order_id = client.next_valid_order_id().await.expect("next id");
    ///     client.submit_order(order_id, &contract, &order).await.expect("submit failed");
    /// }
    /// ```
    pub async fn submit_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &Order) -> Result<(), Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self, order, checked_id.value())?;
        verify::verify_order_contract(self, contract, checked_id.value())?;

        let request = encoders::encode_place_order(checked_id.value(), contract, order)?;
        self.send_message(request).await?;

        Ok(())
    }

    /// Submits an Order with a subscription for updates.
    ///
    /// # Errors
    /// [`Error::OrderIdInRequestRange`] if `order_id`, or a `parent_id` or preset attached-order id on `order`, is at or above 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let order = client
    ///         .order(&contract)
    ///         .buy(100)
    ///         .market()
    ///         .build()
    ///         .expect("order build");
    ///     let order_id = client.next_valid_order_id().await.expect("next id");
    ///     let subscription = client.place_order(order_id, &contract, &order).await.expect("place");
    ///     let mut updates = subscription.filter_data();
    ///     while let Some(update) = updates.next().await {
    ///         println!("{update:?}");
    ///     }
    /// }
    /// ```
    pub async fn place_order(&self, order_id: impl Into<OrderId>, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error> {
        let checked_id = verify::verify_order_ids(order_id.into(), order)?;
        verify::verify_order(self, order, checked_id.value())?;
        verify::verify_order_contract(self, contract, checked_id.value())?;

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
        if !manual_order_cancel_time.is_empty() {
            check_version(self.server_version(), Features::MANUAL_ORDER_TIME)?;
        }

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
        check_version(self.server_version(), Features::REQ_GLOBAL_CANCEL)?;

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
    /// [`Error::OrderIdInRequestRange`] if TWS returns an id at or above 1,500,000,000 (reserved for request ids).
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
        check_version(self.server_version(), Features::COMPLETED_ORDERS)?;

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

    /// Requests status updates about future orders placed from TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
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
