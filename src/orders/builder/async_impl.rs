use super::order_builder::{AttachedOrdersBuilder, BracketOrderBuilder, ClientBound, OrderBuilder};
use super::types::{AttachedOrderIds, BracketOrderIds};
use crate::client::r#async::Client;
use crate::errors::Error;
use crate::orders::{Action, OrderId, OrderType};

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

    /// Analyze the order for margin and commission (what-if): TWS evaluates it without placing it.
    ///
    /// Returns the order state that carries the margins and commission. TWS answers some orders
    /// (live: US stock limit orders) with a warning-only frame first; the returned state keeps
    /// that frame's [`warning_text`](crate::orders::OrderState::warning_text) (live: 2026-11-24,
    /// server 223; `docs/wire-facts/orders.md#what-if-answer-frames`). For an order built
    /// elsewhere, use [`Client::analyze_order`].
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, or
    /// [`Error::InvalidArgument`] for a required input the typestate cannot validate (see
    /// [`OrderBuilder::build`]), before anything is sent;
    /// [`Error::OrderIdInRequestRange`] if the client's next order id has reached 1,500,000,000
    /// (reserved for request ids); the error TWS returns if it rejects the what-if order,
    /// including the 201 for an order the account's margin cannot carry;
    /// [`Error::UnexpectedEndOfStream`] if the reply ends without the order's state;
    /// [`Error::Timeout`] if the state has not come within the client's one-shot timeout (30 s
    /// unless set), as for [`Client::analyze_order`].
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
    ///     let state = client.order(&contract).buy(100).limit(150.0).analyze().await.expect("analyze failed");
    ///     println!(
    ///         "initial margin {:?} -> {:?}, commission {:?} {}",
    ///         state.initial_margin_before, state.initial_margin_after, state.commission_and_fees, state.commission_and_fees_currency
    ///     );
    /// }
    /// ```
    pub async fn analyze(self) -> Result<crate::orders::OrderState, Error> {
        let ClientBound { client, contract } = self.target;
        let order = self.build()?;
        client.analyze_order(client.next_order_id(), contract, &order).await
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
    /// the error's `sent` ids (live: 2026-11-27, paper Gateway, server 226). If the parent's send
    /// is refused while the client reconnects, nothing was sent, and the error is
    /// [`Error::Reconnecting`].
    ///
    /// Dropping the returned future between sends (a timeout, a `select!` branch) also stops
    /// partway, and returns nothing, so held legs it sent can't be found by id: avoid racing it
    /// against a timeout. Without one, a write stalled on a peer that stopped reading blocks the
    /// call until the client finds the connection lost and severs it, which fails the write.
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
