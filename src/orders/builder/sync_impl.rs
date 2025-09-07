use super::{BracketOrderBuilder, BracketOrderIds, OrderBuilder, OrderId};
use crate::client::sync::Client;
use crate::contracts::Contract;
use crate::errors::Error;
use crate::orders;

#[cfg(test)]
mod tests;

impl<'a> OrderBuilder<'a, Client> {
    /// Submit the order synchronously
    /// Returns the order ID assigned to the submitted order
    pub fn submit(self) -> Result<OrderId, Error> {
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;
        orders::submit_order(client, order_id, contract, &order)?;
        Ok(OrderId::new(order_id))
    }

    /// Build the order and return it without submitting
    /// Useful for batch operations or custom submission logic
    pub fn build_order(self) -> Result<crate::orders::Order, Error> {
        self.build().map_err(Into::into)
    }

    /// Analyze order for margin/commission (what-if)
    pub fn analyze(mut self) -> Result<crate::orders::OrderState, Error> {
        self.what_if = true;
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;

        // Submit what-if order and get the response
        let responses = orders::place_order(client, order_id, contract, &order)?;

        // Look for the order state in the responses
        for response in responses {
            if let crate::orders::PlaceOrder::OpenOrder(order_data) = response {
                if order_data.order_id == order_id {
                    return Ok(order_data.order_state);
                }
            }
        }

        Err(Error::Simple("What-if analysis did not return order state".to_string()))
    }
}

impl<'a> BracketOrderBuilder<'a, Client> {
    /// Submit bracket orders synchronously
    /// Returns BracketOrderIds containing all three order IDs
    pub fn submit_all(self) -> Result<BracketOrderIds, Error> {
        let client = self.parent_builder.client;
        let contract = self.parent_builder.contract;
        let base_id = client.next_order_id();
        let orders = self.build()?;

        let mut order_ids = Vec::new();

        for (i, mut order) in orders.into_iter().enumerate() {
            let order_id = base_id + i as i32;
            order.order_id = order_id;
            order_ids.push(order_id);

            // Update parent_id for child orders
            if i > 0 {
                order.parent_id = base_id;
            }

            // Only transmit the last order
            if i == 2 {
                order.transmit = true;
            }

            orders::submit_order(client, order_id, contract, &order)?;
        }

        Ok(BracketOrderIds::new(order_ids[0], order_ids[1], order_ids[2]))
    }
}

/// Extension trait for submitting multiple OCA orders
impl Client {
    /// Submit multiple OCA (One-Cancels-All) orders
    ///
    /// When one order in the group is filled, all others are automatically cancelled.
    ///
    /// # Example
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract1 = Contract::stock("AAPL").build();
    /// let contract2 = Contract::stock("MSFT").build();
    ///
    /// let order1 = client.order(&contract1)
    ///     .buy(100)
    ///     .limit(50.0)
    ///     .oca_group("MyOCA", 1)
    ///     .build_order().expect("order build failed");
    ///     
    /// let order2 = client.order(&contract2)
    ///     .buy(100)
    ///     .limit(45.0)
    ///     .oca_group("MyOCA", 1)
    ///     .build_order().expect("order build failed");
    ///
    /// let order_ids = client.submit_oca_orders(
    ///     vec![(contract1, order1), (contract2, order2)]
    /// ).expect("OCA submission failed");
    /// ```
    pub fn submit_oca_orders(&self, orders: Vec<(Contract, crate::orders::Order)>) -> Result<Vec<OrderId>, Error> {
        let mut order_ids = Vec::new();
        let base_id = self.next_order_id();

        for (i, (contract, mut order)) in orders.into_iter().enumerate() {
            let order_id = base_id + i as i32;
            order.order_id = order_id;
            order_ids.push(OrderId::new(order_id));
            orders::submit_order(self, order_id, &contract, &order)?;
        }

        Ok(order_ids)
    }
}
