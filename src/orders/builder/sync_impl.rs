use super::order_builder::{AttachedOrdersBuilder, BracketOrderBuilder, ClientBound, OrderBuilder};
use super::types::{AttachedOrderIds, BracketOrderIds};
use crate::client::sync::Client;
use crate::errors::Error;
use crate::orders::OrderId;
use crate::orders::PlaceOrder;
impl OrderBuilder<ClientBound<'_, Client>> {
    /// Submit the order synchronously
    /// Returns the order ID assigned to the submitted order
    pub fn submit(self) -> Result<OrderId, Error> {
        let ClientBound { client, contract } = self.target;
        let order_id = client.next_order_id();
        let order = self.build()?;
        client.submit_order(order_id, contract, &order)?;
        Ok(OrderId::new(order_id))
    }

    /// Analyze order for margin/commission (what-if)
    pub fn analyze(mut self) -> Result<crate::orders::OrderState, Error> {
        self.what_if = true;
        let ClientBound { client, contract } = self.target;
        let order_id = client.next_order_id();
        let order = self.build()?;

        // Submit what-if order and get the response
        let responses = client.place_order(order_id, contract, &order)?;

        // Look for the order state in the responses. `?` propagates a rejected
        // what-if order; the earlier `if let Ok(..)` read discarded it, so the
        // caller saw `UnexpectedEndOfStream` instead of TWS's reason (#735).
        for response in responses.iter_data() {
            if let PlaceOrder::OpenOrder(order_data) = response? {
                if order_data.order_id == order_id {
                    return Ok(order_data.order_state);
                }
            }
        }

        Err(Error::UnexpectedEndOfStream)
    }
}

impl BracketOrderBuilder<'_, Client> {
    /// Submit bracket orders synchronously
    /// Returns BracketOrderIds containing all three order IDs
    pub fn submit_all(self) -> Result<BracketOrderIds, Error> {
        let ClientBound { client, contract } = self.parent_builder.target;
        let orders = self.build()?;

        // Reserve all order IDs upfront to prevent collisions
        let parent_id = client.next_order_id();
        let tp_id = client.next_order_id();
        let sl_id = client.next_order_id();
        let reserved_ids = [parent_id, tp_id, sl_id];

        for (i, mut order) in orders.into_iter().enumerate() {
            let order_id = reserved_ids[i];
            order.order_id = order_id;

            // Update parent_id for child orders
            if i > 0 {
                order.parent_id = parent_id;
            }

            // Only transmit the last order
            if i == 2 {
                order.transmit = true;
            }

            client.submit_order(order_id, contract, &order)?;
        }

        Ok(BracketOrderIds::new(parent_id, tp_id, sl_id))
    }
}

impl AttachedOrdersBuilder<'_, Client> {
    /// Submit the order with its preset children synchronously.
    ///
    /// Allocates the parent id, then one id per requested child, and sends a single
    /// place-order request. Fire-and-forget: whether TWS attached the children shows up only
    /// on the order update stream — see [`Order::preset_stop_loss_order_id`](crate::orders::Order::preset_stop_loss_order_id).
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
