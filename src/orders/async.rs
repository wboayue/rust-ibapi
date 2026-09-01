//! Asynchronous implementation of order management functionality

use time::OffsetDateTime;

use crate::common::request_helpers::{self, expect_proto};
use crate::messages::OutgoingMessages;
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
    pub fn order<'a>(&'a self, contract: &'a Contract) -> OrderBuilder<'a, Self> {
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
    /// `execution_id` — the two arrive in either order but share that key. See
    /// the [`CommissionReport`] docs for the idiom.
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
        Ok(Subscription::new_from_internal_simple::<OrderUpdate>(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Submits an Order (fire-and-forget).
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
    pub async fn submit_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(order_id, contract, order)?;
        self.send_message(request).await?;

        Ok(())
    }

    /// Submits an Order with a subscription for updates.
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
    pub async fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(order_id, contract, order)?;
        let internal_subscription = self.send_order(order_id, request).await?;

        Ok(Subscription::new_from_internal_simple::<PlaceOrder>(
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
    pub async fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        if !manual_order_cancel_time.is_empty() {
            check_version(self.server_version(), Features::MANUAL_ORDER_TIME)?;
        }

        let request = encoders::encode_cancel_order(order_id, manual_order_cancel_time)?;
        let internal_subscription = self.send_order(order_id, request).await?;

        Ok(Subscription::new_from_internal_simple::<CancelOrder>(
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

        self.set_next_order_id(next_order_id);
        Ok(next_order_id)
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
        Ok(Subscription::new_from_internal_simple::<Orders>(
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
        Ok(Subscription::new_from_internal_simple::<Orders>(
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
        Ok(Subscription::new_from_internal_simple::<Orders>(
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
        Ok(Subscription::new_from_internal_simple::<Orders>(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Requests current day's executions matching the filter.
    ///
    /// Both [`ExecutionData`] and
    /// [`CommissionReport`] are delivered on this
    /// stream. Join a commission to its execution deterministically by `execution_id`
    /// (see the [`CommissionReport`] docs) — the two
    /// may arrive in either order.
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
        let request_id = self.next_request_id();
        let request = encoders::encode_executions(request_id, &filter)?;
        let internal_subscription = self.send_request(request_id, request).await?;
        Ok(Subscription::new_from_internal_simple::<Executions>(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }

    /// Exercise an option contract.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::orders::ExerciseAction;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::option("AAPL", "20251219", 150.0, OptionRight::Call);
    ///     let subscription = client
    ///         .exercise_options(&contract, ExerciseAction::Exercise, 1, "DU000001", false, None)
    ///         .await
    ///         .expect("exercise_options failed");
    ///     // Consume the subscription so execution updates and commission reports surface.
    ///     let mut events = subscription.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         match event {
    ///             Ok(item) => println!("exercise event: {item:?}"),
    ///             Err(e)   => { eprintln!("exercise err: {e:?}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn exercise_options(
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
        let internal_subscription = self.send_order(order_id, request).await?;
        Ok(Subscription::new_from_internal_simple::<ExerciseOptions>(
            internal_subscription,
            self.message_bus.clone(),
            self.decoder_context(),
        ))
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
