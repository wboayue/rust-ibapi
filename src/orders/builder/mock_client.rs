//! Mock client for testing OrderBuilder functionality
#[cfg(test)]
pub mod mock {
    use std::sync::{Arc, Mutex};
    use crate::contracts::Contract;
    use crate::errors::Error;
    use crate::orders::{Order, OrderState, PlaceOrder, OrderData};

    /// Mock client for testing OrderBuilder
    pub struct MockOrderClient {
        next_order_id: Arc<Mutex<i32>>,
        submitted_orders: Arc<Mutex<Vec<(i32, Contract, Order)>>>,
        place_order_responses: Arc<Mutex<Vec<Vec<PlaceOrder>>>>,
        submit_order_responses: Arc<Mutex<Vec<Result<(), Error>>>>,
        oca_orders: Arc<Mutex<Vec<Vec<(Contract, Order)>>>>,
    }

    impl MockOrderClient {
        pub fn new() -> Self {
            Self {
                next_order_id: Arc::new(Mutex::new(100)),
                submitted_orders: Arc::new(Mutex::new(Vec::new())),
                place_order_responses: Arc::new(Mutex::new(Vec::new())),
                submit_order_responses: Arc::new(Mutex::new(Vec::new())),
                oca_orders: Arc::new(Mutex::new(Vec::new())),
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

        pub fn get_submitted_orders(&self) -> Vec<(i32, Contract, Order)> {
            self.submitted_orders.lock().unwrap().clone()
        }

        pub fn get_oca_orders(&self) -> Vec<Vec<(Contract, Order)>> {
            self.oca_orders.lock().unwrap().clone()
        }
    }

    // Implement the minimal client interface needed by OrderBuilder
    impl MockOrderClient {
        pub fn next_order_id(&self) -> i32 {
            let mut id = self.next_order_id.lock().unwrap();
            let current = *id;
            *id += 1;
            current
        }

        pub fn submit_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error> {
            self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
            
            let mut responses = self.submit_order_responses.lock().unwrap();
            if !responses.is_empty() {
                responses.remove(0)
            } else {
                Ok(())
            }
        }

        pub fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<Vec<PlaceOrder>, Error> {
            self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
            
            let mut responses = self.place_order_responses.lock().unwrap();
            if !responses.is_empty() {
                Ok(responses.remove(0))
            } else {
                // Default response for what-if orders
                if order.what_if {
                    let mut order_state = OrderState::default();
                    order_state.commission = Some(1.50);
                    order_state.initial_margin_before = Some(10000.00);
                    order_state.initial_margin_after = Some(15000.00);
                    
                    Ok(vec![PlaceOrder::OpenOrder(OrderData {
                        order_id,
                        contract: contract.clone(),
                        order: order.clone(),
                        order_state,
                    })])
                } else {
                    Ok(vec![])
                }
            }
        }

        pub fn submit_oca_orders(&self, orders: Vec<(Contract, Order)>) -> Result<Vec<crate::orders::builder::OrderId>, Error> {
            self.oca_orders.lock().unwrap().push(orders.clone());
            
            let base_id = self.next_order_id();
            let mut order_ids = Vec::new();
            
            for (i, (contract, mut order)) in orders.into_iter().enumerate() {
                let order_id = base_id + i as i32;
                order.order_id = order_id;
                order_ids.push(crate::orders::builder::OrderId::new(order_id));
                self.submit_order(order_id, &contract, &order)?;
            }
            
            Ok(order_ids)
        }
    }
}

// Helper functions to work with both sync and async implementations
#[cfg(test)]
pub mod helpers {
    use super::mock::MockOrderClient;
    use crate::orders::{self, Order, OrderState};
    use crate::contracts::Contract;
    use crate::errors::Error;

    // Mock the submit_order function used by sync implementation
    pub fn submit_order(
        client: &MockOrderClient,
        order_id: i32,
        contract: &Contract,
        order: &Order,
    ) -> Result<(), Error> {
        client.submit_order(order_id, contract, order)
    }

    // Mock the place_order function used by sync implementation
    pub fn place_order(
        client: &MockOrderClient,
        order_id: i32,
        contract: &Contract,
        order: &Order,
    ) -> Result<Vec<orders::PlaceOrder>, Error> {
        client.place_order(order_id, contract, order)
    }
}