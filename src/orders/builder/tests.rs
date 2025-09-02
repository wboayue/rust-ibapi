// Integration tests for the order builder module
// These tests span multiple modules and test interaction between components

#[cfg(all(test, feature = "sync"))]
mod sync_integration_tests {
    use super::mock_client::mock::MockOrderClient;
    use crate::contracts::Contract;
    use crate::orders::builder::OrderBuilder;
    use crate::orders::Action;

    fn create_stock_contract(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: crate::contracts::SecurityType::Stock,
            exchange: "SMART".to_string(),
            currency: "USD".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_full_order_workflow() {
        let client = MockOrderClient::new();
        let contract = create_stock_contract("AAPL");

        // Create multiple orders
        let orders = vec![
            OrderBuilder::new(&client, &contract).buy(100).market().build().unwrap(),
            OrderBuilder::new(&client, &contract).sell(50).limit(150.00).build().unwrap(),
            OrderBuilder::new(&client, &contract).buy(200).stop_limit(145.00, 148.00).build().unwrap(),
        ];

        // Verify orders have correct properties
        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[0].order_type, "MKT");
        assert_eq!(orders[0].total_quantity, 100.0);

        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[1].order_type, "LMT");
        assert_eq!(orders[1].limit_price, Some(150.00));

        assert_eq!(orders[2].order_type, "STP LMT");
        assert_eq!(orders[2].aux_price, Some(145.00));
        assert_eq!(orders[2].limit_price, Some(148.00));
    }

    #[test]
    fn test_complex_order_combinations() {
        let client = MockOrderClient::new();
        let contract = create_stock_contract("MSFT");

        // Test complex order with multiple attributes
        let order = OrderBuilder::new(&client, &contract)
            .buy(100)
            .limit(50.00)
            .hidden()
            .outside_rth()
            .good_till_date("20240630 23:59:59")
            .account("TEST123")
            .algo("VWAP")
            .algo_param("startTime", "09:30:00")
            .algo_param("endTime", "16:00:00")
            .oca_group("TestGroup", 1)
            .build()
            .unwrap();

        assert_eq!(order.order_type, "LMT");
        assert_eq!(order.limit_price, Some(50.00));
        assert!(order.hidden);
        assert!(order.outside_rth);
        assert_eq!(order.tif, "GTD");
        assert_eq!(order.good_till_date, "20240630 23:59:59");
        assert_eq!(order.account, "TEST123");
        assert_eq!(order.algo_strategy, "VWAP");
        assert_eq!(order.algo_params.len(), 2);
        assert_eq!(order.oca_group, "TestGroup");
        assert_eq!(order.oca_type, 1);
    }
}

#[cfg(all(test, feature = "async"))]
mod async_integration_tests {
    use super::async_mock_client::mock::AsyncMockClient;
    use crate::contracts::Contract;
    use crate::orders::builder::OrderBuilder;
    use crate::orders::Action;

    fn create_stock_contract(symbol: &str) -> Contract {
        Contract {
            symbol: symbol.to_string(),
            security_type: crate::contracts::SecurityType::Stock,
            exchange: "SMART".to_string(),
            currency: "USD".to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_async_full_order_workflow() {
        let client = AsyncMockClient::new();
        let contract = create_stock_contract("AAPL");

        // Create multiple orders
        let orders = vec![
            OrderBuilder::new(&client, &contract).buy(100).market().build().unwrap(),
            OrderBuilder::new(&client, &contract).sell(50).limit(150.00).build().unwrap(),
            OrderBuilder::new(&client, &contract).buy(200).stop_limit(145.00, 148.00).build().unwrap(),
        ];

        // Verify orders have correct properties
        assert_eq!(orders[0].action, Action::Buy);
        assert_eq!(orders[0].order_type, "MKT");
        assert_eq!(orders[0].total_quantity, 100.0);

        assert_eq!(orders[1].action, Action::Sell);
        assert_eq!(orders[1].order_type, "LMT");
        assert_eq!(orders[1].limit_price, Some(150.00));

        assert_eq!(orders[2].order_type, "STP LMT");
        assert_eq!(orders[2].aux_price, Some(145.00));
        assert_eq!(orders[2].limit_price, Some(148.00));
    }
}

// Mock client implementations for testing
#[cfg(test)]
pub mod mock_client {
    pub mod mock {
        use crate::contracts::Contract;
        use crate::errors::Error;
        use crate::orders::{Order, PlaceOrder};
        use std::sync::{Arc, Mutex};

        #[allow(dead_code)]
        type OcaOrderList = Vec<Vec<(Contract, Order)>>;

        /// Mock client for testing OrderBuilder
        #[allow(dead_code)]
        pub struct MockOrderClient {
            next_order_id: Arc<Mutex<i32>>,
            submitted_orders: Arc<Mutex<Vec<(i32, Contract, Order)>>>,
            place_order_responses: Arc<Mutex<Vec<Vec<PlaceOrder>>>>,
            submit_order_responses: Arc<Mutex<Vec<Result<(), Error>>>>,
            oca_orders: Arc<Mutex<OcaOrderList>>,
        }

        impl Default for MockOrderClient {
            fn default() -> Self {
                Self::new()
            }
        }

        #[allow(dead_code)]
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

            pub fn next_order_id(&self) -> i32 {
                let mut id = self.next_order_id.lock().unwrap();
                let current = *id;
                *id += 1;
                current
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
                    Ok(vec![])
                }
            }

            pub fn submit_oca_orders(&self, orders: Vec<(Contract, Order)>) -> Result<Vec<crate::orders::builder::OrderId>, Error> {
                let mut order_ids = Vec::new();
                for (contract, order) in orders.iter() {
                    let order_id = self.next_order_id();
                    order_ids.push(crate::orders::builder::OrderId::new(order_id));
                    self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
                }
                self.oca_orders.lock().unwrap().push(orders);
                Ok(order_ids)
            }
        }
    }
}

#[cfg(all(test, feature = "async"))]
pub mod async_mock_client {
    pub mod mock {
        use crate::contracts::Contract;
        use crate::errors::Error;
        use crate::orders::{Order, OrderUpdate, PlaceOrder};
        use futures::stream::{self, Stream};
        use std::pin::Pin;
        use std::sync::{Arc, Mutex};

        /// Async mock client for testing OrderBuilder
        pub struct AsyncMockClient {
            next_order_id: Arc<Mutex<i32>>,
            submitted_orders: Arc<Mutex<Vec<(i32, Contract, Order)>>>,
            place_order_responses: Arc<Mutex<Vec<Vec<PlaceOrder>>>>,
            submit_order_responses: Arc<Mutex<Vec<Result<(), Error>>>>,
            order_update_streams: Arc<Mutex<Vec<Vec<OrderUpdate>>>>,
        }

        impl Default for AsyncMockClient {
            fn default() -> Self {
                Self::new()
            }
        }

        impl AsyncMockClient {
            pub fn new() -> Self {
                Self {
                    next_order_id: Arc::new(Mutex::new(100)),
                    submitted_orders: Arc::new(Mutex::new(Vec::new())),
                    place_order_responses: Arc::new(Mutex::new(Vec::new())),
                    submit_order_responses: Arc::new(Mutex::new(Vec::new())),
                    order_update_streams: Arc::new(Mutex::new(Vec::new())),
                }
            }

            pub fn set_next_order_id(&self, id: i32) {
                *self.next_order_id.lock().unwrap() = id;
            }

            pub fn next_order_id(&self) -> i32 {
                let mut id = self.next_order_id.lock().unwrap();
                let current = *id;
                *id += 1;
                current
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
                let items = if !responses.is_empty() {
                    responses.remove(0).into_iter().map(Ok).collect::<Vec<_>>()
                } else {
                    vec![]
                };

                Ok(Box::pin(stream::iter(items)))
            }

            pub async fn submit_order_with_updates(
                &self,
                order_id: i32,
                contract: &Contract,
                order: &Order,
            ) -> Result<Pin<Box<dyn Stream<Item = OrderUpdate> + Send>>, Error> {
                self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));

                let mut streams = self.order_update_streams.lock().unwrap();
                let updates = if !streams.is_empty() { streams.remove(0) } else { vec![] };

                Ok(Box::pin(stream::iter(updates)))
            }

            pub async fn submit_oca_orders(&self, orders: Vec<(Contract, Order)>) -> Result<Vec<crate::orders::builder::OrderId>, Error> {
                let mut order_ids = Vec::new();
                for (contract, order) in orders.iter() {
                    let order_id = self.next_order_id();
                    order_ids.push(crate::orders::builder::OrderId::new(order_id));
                    self.submitted_orders.lock().unwrap().push((order_id, contract.clone(), order.clone()));
                }
                Ok(order_ids)
            }
        }
    }
}
