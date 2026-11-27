use futures::StreamExt;

use super::order_builder::{AttachedOrdersBuilder, BracketOrderBuilder, ClientBound, OrderBuilder};
use super::types::{AttachedOrderIds, BracketOrderIds};
use crate::client::r#async::Client;
use crate::errors::Error;
use crate::orders::PlaceOrder;
use crate::orders::{Action, OrderId, OrderType};
use crate::subscriptions::SubscriptionItemStreamExt;

impl OrderBuilder<ClientBound<'_, Client>, Action, OrderType> {
    /// Submit the order asynchronously
    /// Returns the order ID assigned to the submitted order
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, before anything is sent;
    /// [`Error::InvalidArgument`] for a required input the typestate can't see (see
    /// [`OrderBuilder::build`]); [`Error::OrderIdInRequestRange`] if the client's next order id
    /// has reached 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// `submit` needs a side and an order type. Without the order type:
    ///
    /// ```compile_fail,E0599
    /// # async fn run(client: &ibapi::Client) {
    /// # let contract = ibapi::contracts::Contract::stock("AAPL").build();
    /// let order_id = client.order(&contract).buy(100).submit();
    /// # }
    /// ```
    ///
    /// Without the side:
    ///
    /// ```compile_fail,E0599
    /// # async fn run(client: &ibapi::Client) {
    /// # let contract = ibapi::contracts::Contract::stock("AAPL").build();
    /// let order_id = client.order(&contract).market().submit();
    /// # }
    /// ```
    pub async fn submit(self) -> Result<i32, Error> {
        let ClientBound { client, contract } = self.target;
        let order_id = client.next_order_id();
        let order = self.build()?;
        client.submit_order(order_id, contract, &order).await?;
        Ok(order_id)
    }

    /// Analyze order for margin/commission (what-if)
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, before anything is sent;
    /// [`Error::InvalidArgument`] for a required input the typestate can't see (see
    /// [`OrderBuilder::build`]); [`Error::OrderIdInRequestRange`] if the client's next order id
    /// has reached 1,500,000,000 (reserved for request ids); the error TWS returns if it rejects the what-if order;
    /// [`Error::UnexpectedEndOfStream`] if the reply ends without the order's state.
    pub async fn analyze(mut self) -> Result<crate::orders::OrderState, Error> {
        self.fields.what_if = true;
        let ClientBound { client, contract } = self.target;
        let order_id = client.next_order_id();
        let order = self.build()?;

        // Submit what-if order and get the response
        let subscription = client.place_order(order_id, contract, &order).await?;

        // Look for the order state in the responses. `?` propagates a rejected
        // what-if order; the earlier `while let Some(Ok(..))` read discarded it by
        // ending the loop, so the caller saw `UnexpectedEndOfStream` (#735).
        let mut data = subscription.filter_data();
        while let Some(response) = data.next().await {
            if let PlaceOrder::OpenOrder(order_data) = response? {
                if order_data.order_id == order_id {
                    return Ok(order_data.order_state);
                }
            }
        }

        Err(Error::UnexpectedEndOfStream)
    }
}

impl BracketOrderBuilder<'_, Client, OrderType, f64, f64> {
    /// Submit bracket orders asynchronously
    /// Returns BracketOrderIds containing all three order IDs
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, before anything is sent;
    /// [`Error::InvalidArgument`] for a required input the typestate can't see (see
    /// [`OrderBuilder::build`]); [`Error::OrderIdInRequestRange`] if an allocated id reaches
    /// 1,500,000,000 (reserved for request ids); [`Error::ServerVersion`] if the order sets a
    /// field the connected server doesn't support. None of these sends anything: all three
    /// orders are checked and encoded before the first is sent.
    ///
    /// Only a transport error stops partway, and returns [`Error::OrdersPartiallySent`] with the
    /// ids of the legs that may be at TWS. A parent and take-profit sent before the stop-loss are
    /// held there (they transmit with it). After a reconnect with the same client id TWS still
    /// holds them without listing them in `open_orders`, so cancel them with `cancel_order` and
    /// the error's `sent` ids (live: 2026-11-27, paper Gateway, server 226).
    ///
    /// Dropping the returned future between sends (a timeout, a `select!` branch) also stops
    /// partway, and returns nothing, so held legs it sent can't be found by id. Let `submit_all`
    /// finish rather than racing it against a timeout.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::contracts::Contract;
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .bracket()
    ///     .entry_limit(150.0)
    ///     .take_profit(160.0)
    ///     .stop_loss(145.0)
    ///     .submit_all()
    ///     .await?;
    /// println!("parent {} take-profit {} stop-loss {}", ids.parent, ids.take_profit, ids.stop_loss);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// `submit_all` needs the entry, the take-profit and the stop-loss. Without the entry:
    ///
    /// ```compile_fail,E0599
    /// # async fn run(client: &ibapi::Client) {
    /// # let contract = ibapi::contracts::Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).bracket().take_profit(160.0).stop_loss(145.0).submit_all();
    /// # }
    /// ```
    ///
    /// Without the take-profit:
    ///
    /// ```compile_fail,E0599
    /// # async fn run(client: &ibapi::Client) {
    /// # let contract = ibapi::contracts::Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).bracket().entry_limit(150.0).stop_loss(145.0).submit_all();
    /// # }
    /// ```
    ///
    /// Without the stop-loss:
    ///
    /// ```compile_fail,E0599
    /// # async fn run(client: &ibapi::Client) {
    /// # let contract = ibapi::contracts::Contract::stock("AAPL").build();
    /// let ids = client.order(&contract).buy(100).bracket().entry_market().take_profit(160.0).submit_all();
    /// # }
    /// ```
    pub async fn submit_all(self) -> Result<BracketOrderIds, Error> {
        let ClientBound { client, contract } = self.parent_builder.target;
        let (orders, ids) = self.build_with_ids(|| client.next_order_id())?;
        client
            .submit_orders(orders.iter().map(|order| (OrderId::from(order.order_id), contract, order)))
            .await?;
        Ok(ids)
    }
}

impl AttachedOrdersBuilder<'_, Client> {
    /// Submit the order with its preset children asynchronously.
    ///
    /// Allocates the parent id, then one id per requested child, and sends a single
    /// place-order request. Fire-and-forget: whether TWS attached the children shows up only
    /// on the order update stream — see [`Order::preset_stop_loss_order_id`](crate::orders::Order::preset_stop_loss_order_id).
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, before anything is sent;
    /// [`Error::InvalidArgument`] for a required input the typestate can't see (see
    /// [`OrderBuilder::build`]); [`Error::OrderIdInRequestRange`] if the client's next order id
    /// has reached 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("AAPL").build();
    ///     let ids = client
    ///         .order(&contract)
    ///         .buy(100)
    ///         .limit(150.0)
    ///         .preset_stop_loss()
    ///         .preset_profit_taker()
    ///         .submit()
    ///         .await
    ///         .expect("submit failed");
    ///     println!("parent {} stop-loss {:?} profit-taker {:?}", ids.parent, ids.stop_loss, ids.profit_taker);
    /// }
    /// ```
    pub async fn submit(self) -> Result<AttachedOrderIds, Error> {
        let ClientBound { client, contract } = self.parent_builder.target;
        let (order, ids) = self.build_with_ids(|| client.next_order_id())?;
        client.submit_order(order.order_id, contract, &order).await?;
        Ok(ids)
    }
}
