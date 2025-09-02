//! Async mock client for testing OrderBuilder async functionality
#[cfg(all(test, feature = "async"))]
pub mod mock {
    use std::sync::{Arc, Mutex};
    use crate::contracts::Contract;
    use crate::errors::Error;
    use crate::orders::{Order, OrderState, PlaceOrder, OrderData, OrderUpdate, OrderStatus};
    use futures::stream::{self, Stream, StreamExt};
    use std::pin::Pin;

    /// Async mock client for testing OrderBuilder
    pub struct AsyncMockClient {
        next_order_id: Arc<Mutex<i32>>,
        submitted_orders: Arc<Mutex<Vec<(i32, Contract, Order)>>>,
        place_order_responses: Arc<Mutex<Vec<Vec<PlaceOrder>>>>,
        submit_order_responses: Arc<Mutex<Vec<Result<(), Error>>>>,
        oca_orders: Arc<Mutex<Vec<Vec<(Contract, Order)>>>>,
        order_update_streams: Arc<Mutex<Vec<Vec<OrderUpdate>>>>,
    }

    impl AsyncMockClient {
        pub fn new() -> Self {
            Self {
                next_order_id: Arc::new(Mutex::new(100)),
                submitted_orders: Arc::new(Mutex::new(Vec::new())),
                place_order_responses: Arc::new(Mutex::new(Vec::new())),
                submit_order_responses: Arc::new(Mutex::new(Vec::new())),
                oca_orders: Arc::new(Mutex::new(Vec::new())),
                order_update_streams: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn set_next_order_id(&self, id: i32) {
            *self.next_order_id.lock().unwrap() = id;
        }

        pub fn add_place_order_response(&self, response: Vec<PlaceOrder>) {
            self.place_order_responses.lock().unwrap().push(response);
        }

        pub fn add_submit_order_response(&self, response: Result<(), Error>) {
            self.submit_order_responses.lock().unwrap().push(response);
        }

        pub fn add_order_update_stream(&self, updates: Vec<OrderUpdate>) {
            self.order_update_streams.lock().unwrap().push(updates);
        }

        pub fn get_submitted_orders(&self) -> Vec<(i32, Contract, Order)> {
            self.submitted_orders.lock().unwrap().clone()
        }

        pub fn get_oca_orders(&self) -> Vec<Vec<(Contract, Order)>> {
            self.oca_orders.lock().unwrap().clone()
        }
    }

    // Implement the minimal async client interface needed by OrderBuilder
    impl AsyncMockClient {
        pub fn next_order_id(&self) -> i32 {
            let mut id = self.next_order_id.lock().unwrap();
            let current = *id;
            *id += 1;
            current
        }

        pub async fn submit_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error> {
            self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
            
            let mut responses = self.submit_order_responses.lock().unwrap();
            if !responses.is_empty() {
                responses.remove(0)
            } else {
                Ok(())
            }
        }

        pub async fn place_order(
            &self,
            order_id: i32,
            contract: &Contract,
            order: &Order,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<PlaceOrder, Error>> + Send>>, Error> {
            self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
            
            let mut responses = self.place_order_responses.lock().unwrap();
            let response_vec = if !responses.is_empty() {
                responses.remove(0)
            } else {
                // Default response for what-if orders
                if order.what_if {
                    let mut order_state = OrderState::default();
                    order_state.commission = Some(1.50);
                    order_state.initial_margin_before = Some(10000.00);
                    order_state.initial_margin_after = Some(15000.00);
                    
                    vec![PlaceOrder::OpenOrder(OrderData {
                        order_id,
                        contract: contract.clone(),
                        order: order.clone(),
                        order_state,
                    })]
                } else {
                    vec![]
                }
            };

            let stream = stream::iter(response_vec.into_iter().map(Ok));
            Ok(Box::pin(stream))
        }

        pub async fn submit_oca_orders(&self, orders: Vec<(Contract, Order)>) -> Result<Vec<crate::orders::builder::OrderId>, Error> {
            self.oca_orders.lock().unwrap().push(orders.clone());
            
            let base_id = self.next_order_id();
            let mut order_ids = Vec::new();
            
            for (i, (contract, mut order)) in orders.into_iter().enumerate() {
                let order_id = base_id + i as i32;
                order.order_id = order_id;
                order_ids.push(crate::orders::builder::OrderId::new(order_id));
                self.submit_order(order_id, &contract, &order).await?;
            }
            
            Ok(order_ids)
        }

        pub async fn submit_order_with_updates(
            &self,
            order_id: i32,
            contract: &Contract,
            order: &Order,
        ) -> Result<Pin<Box<dyn Stream<Item = OrderUpdate> + Send>>, Error> {
            self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
            
            let mut update_streams = self.order_update_streams.lock().unwrap();
            let updates = if !update_streams.is_empty() {
                update_streams.remove(0)
            } else {
                // Default update stream
                vec![
                    OrderUpdate::OrderStatus(OrderStatus {
                        order_id,
                        status: "Submitted".to_string(),
                        filled: 0.0,
                        remaining: order.total_quantity,
                        average_fill_price: 0.0,
                        perm_id: 12345,
                        parent_id: 0,
                        last_fill_price: 0.0,
                        client_id: 0,
                        why_held: String::new(),
                        market_cap_price: 0.0,
                    }),
                    OrderUpdate::OrderStatus(OrderStatus {
                        order_id,
                        status: "Filled".to_string(),
                        filled: order.total_quantity,
                        remaining: 0.0,
                        average_fill_price: order.limit_price.unwrap_or(50.0),
                        perm_id: 12345,
                        parent_id: 0,
                        last_fill_price: order.limit_price.unwrap_or(0.0),
                        client_id: 0,
                        why_held: String::new(),
                        market_cap_price: 0.0,
                    }),
                ]
            };

            let stream = stream::iter(updates.into_iter());
            Ok(Box::pin(stream))
        }
    }
}

// Helper functions to work with async implementations
#[cfg(all(test, feature = "async"))]
pub mod helpers {
    use super::mock::AsyncMockClient;
    use crate::orders::{self, Order, OrderUpdate};
    use crate::contracts::Contract;
    use crate::errors::Error;
    use futures::Stream;
    use std::pin::Pin;

    // Mock the submit_order function used by async implementation
    pub async fn submit_order(
        client: &AsyncMockClient,
        order_id: i32,
        contract: &Contract,
        order: &Order,
    ) -> Result<(), Error> {
        client.submit_order(order_id, contract, order).await
    }

    // Mock the place_order function used by async implementation
    pub async fn place_order(
        client: &AsyncMockClient,
        order_id: i32,
        contract: &Contract,
        order: &Order,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<orders::PlaceOrder, Error>> + Send>>, Error> {
        client.place_order(order_id, contract, order).await
    }
}