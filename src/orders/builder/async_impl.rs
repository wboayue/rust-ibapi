use super::{BracketOrderBuilder, BracketOrderIds, OrderBuilder, OrderId};
use crate::client::r#async::Client;
use crate::errors::Error;
use crate::orders;

#[cfg(test)]
mod tests;

impl<'a> OrderBuilder<'a, Client> {
    /// Submit the order asynchronously
    /// Returns the order ID assigned to the submitted order
    pub async fn submit(self) -> Result<OrderId, Error> {
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;
        orders::submit_order(client, order_id, contract, &order).await?;
        Ok(OrderId::new(order_id))
    }

    /// Build the order and return it without submitting
    /// Useful for batch operations or custom submission logic
    pub fn build_order(self) -> Result<crate::orders::Order, Error> {
        self.build().map_err(Into::into)
    }

    /// Analyze order for margin/commission (what-if)
    pub async fn analyze(mut self) -> Result<crate::orders::OrderState, Error> {
        self.what_if = true;
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;

        // Submit what-if order and get the response
        let mut subscription = orders::place_order(client, order_id, contract, &order).await?;

        // Look for the order state in the responses
        while let Some(Ok(response)) = subscription.next().await {
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
    /// Submit bracket orders asynchronously
    /// Returns BracketOrderIds containing all three order IDs
    pub async fn submit_all(self) -> Result<BracketOrderIds, Error> {
        let client = self.parent_builder.client;
        let contract = self.parent_builder.contract;
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

            orders::submit_order(client, order_id, contract, &order).await?;
        }

        Ok(BracketOrderIds::new(parent_id, tp_id, sl_id))
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
    ///         .oca_group("MyOCA", 1)
    ///         .build_order().expect("order build failed");
    ///         
    ///     let order2 = client.order(&contract2)
    ///         .buy(100)
    ///         .limit(45.0)
    ///         .oca_group("MyOCA", 1)
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
            orders::submit_order(self, order_id, &contract, &order).await?;
        }

        Ok(order_ids)
    }
}
