use super::order_builder::{AttachedOrdersBuilder, BracketOrderBuilder, ClientBound, OrderBuilder};
use super::types::{AttachedOrderIds, BracketOrderIds};
use crate::client::sync::Client;
use crate::errors::Error;
use crate::orders::{Action, OrderId, OrderType};
impl OrderBuilder<ClientBound<'_, Client>, Action, OrderType> {
    /// Submit the order synchronously
    /// Returns the order ID assigned to the submitted order
    ///
    /// # Errors
    /// [`Error::Validation`] if the builder's inputs fail validation, before anything is sent;
    /// [`Error::InvalidArgument`] for a required input the typestate can't see (see
    /// [`OrderBuilder::build`]); [`Error::OrderIdInRequestRange`] if the client's next order id
    /// has reached 1,500,000,000 (reserved for request ids).
    pub fn submit(self) -> Result<i32, Error> {
        let ClientBound { client, contract } = self.target;
        let order_id = client.next_order_id();
        let order = self.build()?;
        client.submit_order(order_id, contract, &order)?;
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let state = client.order(&contract).buy(100).limit(150.0).analyze().expect("analyze failed");
    /// println!(
    ///     "initial margin {:?} -> {:?}, commission {:?} {}",
    ///     state.initial_margin_before, state.initial_margin_after, state.commission_and_fees, state.commission_and_fees_currency
    /// );
    /// ```
    pub fn analyze(self) -> Result<crate::orders::OrderState, Error> {
        let ClientBound { client, contract } = self.target;
        let order = self.build()?;
        client.analyze_order(client.next_order_id(), contract, &order)
    }
}

impl BracketOrderBuilder<'_, Client, OrderType, f64, f64> {
    /// Submit bracket orders synchronously
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
    pub fn submit_all(self) -> Result<BracketOrderIds, Error> {
        let ClientBound { client, contract } = self.parent_builder.target;
        let (orders, ids) = self.build_with_ids(|| client.next_order_id())?;
        client.submit_orders(orders.iter().map(|order| (OrderId::from(order.order_id), contract, order)))?;
        Ok(ids)
    }
}

impl AttachedOrdersBuilder<'_, Client> {
    /// Submit the order with its preset children synchronously.
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
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("AAPL").build();
    /// let ids = client
    ///     .order(&contract)
    ///     .buy(100)
    ///     .limit(150.0)
    ///     .preset_stop_loss()
    ///     .preset_profit_taker()
    ///     .submit()
    ///     .expect("submit failed");
    /// println!("parent {} stop-loss {:?} profit-taker {:?}", ids.parent, ids.stop_loss, ids.profit_taker);
    /// ```
    pub fn submit(self) -> Result<AttachedOrderIds, Error> {
        let ClientBound { client, contract } = self.parent_builder.target;
        let (order, ids) = self.build_with_ids(|| client.next_order_id())?;
        client.submit_order(order.order_id, contract, &order)?;
        Ok(ids)
    }
}
