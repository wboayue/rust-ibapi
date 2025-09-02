use crate::contracts::Contract;
use crate::errors::Error;
use crate::orders::builder::tests::mock_client::mock::MockOrderClient;
use crate::orders::builder::{BracketOrderBuilder, BracketOrderIds, OrderBuilder, OrderId};
use crate::orders::{Action, Order, OrderData, OrderState, PlaceOrder};

fn create_stock_contract(symbol: &str) -> Contract {
    Contract {
        symbol: symbol.to_string(),
        security_type: crate::contracts::SecurityType::Stock,
        exchange: "SMART".to_string(),
        currency: "USD".to_string(),
        ..Default::default()
    }
}

// Mock the orders module functions for testing
mod orders {
    use super::*;

    pub fn submit_order(client: &MockOrderClient, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error> {
        client.submit_order(order_id, contract, order)
    }

    pub fn place_order(client: &MockOrderClient, order_id: i32, contract: &Contract, order: &Order) -> Result<Vec<PlaceOrder>, Error> {
        client.place_order(order_id, contract, order)
    }
}

// Now we need to implement the submit method for MockOrderClient
impl<'a> OrderBuilder<'a, MockOrderClient> {
    /// Build the order and return it without submitting
    pub fn build_order(self) -> Result<Order, Error> {
        self.build().map_err(Into::into)
    }

    /// Submit the order using mock client
    pub fn submit(self) -> Result<OrderId, Error> {
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;
        orders::submit_order(client, order_id, contract, &order)?;
        Ok(OrderId::new(order_id))
    }

    /// Analyze order for margin/commission (what-if)
    pub fn analyze(mut self) -> Result<OrderState, Error> {
        self.what_if = true;
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;

        // Submit what-if order and get the response
        let responses = orders::place_order(client, order_id, contract, &order)?;

        // Look for the order state in the responses
        for response in responses {
            if let PlaceOrder::OpenOrder(order_data) = response {
                if order_data.order_id == order_id {
                    return Ok(order_data.order_state);
                }
            }
        }

        Err(Error::Simple("What-if analysis did not return order state".to_string()))
    }
}

// Implement submit_all for bracket orders
impl<'a> BracketOrderBuilder<'a, MockOrderClient> {
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

#[test]
fn test_order_submit() {
    let client = MockOrderClient::new();
    let contract = create_stock_contract("AAPL");

    client.set_next_order_id(100);

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let order_id = builder.submit().unwrap();
    assert_eq!(order_id.value(), 100);

    // Verify the order was submitted
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].0, 100);
    assert_eq!(submitted[0].2.action, Action::Buy);
    assert_eq!(submitted[0].2.total_quantity, 100.0);
    assert_eq!(submitted[0].2.order_type, "LMT");
    assert_eq!(submitted[0].2.limit_price, Some(50.00));
}

#[test]
fn test_order_analyze() {
    let client = MockOrderClient::new();
    let contract = create_stock_contract("AAPL");

    // Set up a custom response for what-if analysis
    let order_state = OrderState {
        commission: Some(2.50),
        initial_margin_before: Some(20000.00),
        initial_margin_after: Some(25000.00),
        ..Default::default()
    };

    client.add_place_order_response(vec![PlaceOrder::OpenOrder(OrderData {
        order_id: 100,
        contract: contract.clone(),
        order: Default::default(),
        order_state: order_state.clone(),
    })]);

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let analysis = builder.analyze().unwrap();
    assert_eq!(analysis.commission, Some(2.50));
    assert_eq!(analysis.initial_margin_before, Some(20000.00));
    assert_eq!(analysis.initial_margin_after, Some(25000.00));

    // Verify what-if flag was set
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 1);
    assert!(submitted[0].2.what_if);
}

#[test]
fn test_bracket_order_submit_all() {
    let client = MockOrderClient::new();
    let contract = create_stock_contract("AAPL");

    client.set_next_order_id(200);

    let bracket_builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let bracket_ids = bracket_builder.submit_all().unwrap();
    assert_eq!(bracket_ids.parent.value(), 200);
    assert_eq!(bracket_ids.take_profit.value(), 201);
    assert_eq!(bracket_ids.stop_loss.value(), 202);

    // Verify three orders were submitted
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 3);

    // Check parent order
    assert_eq!(submitted[0].0, 200);
    assert_eq!(submitted[0].2.action, Action::Buy);
    assert_eq!(submitted[0].2.order_type, "LMT");
    assert_eq!(submitted[0].2.limit_price, Some(50.0));
    assert!(!submitted[0].2.transmit);

    // Check take profit order
    assert_eq!(submitted[1].0, 201);
    assert_eq!(submitted[1].2.action, Action::Sell);
    assert_eq!(submitted[1].2.order_type, "LMT");
    assert_eq!(submitted[1].2.limit_price, Some(55.0));
    assert_eq!(submitted[1].2.parent_id, 200);
    assert!(!submitted[1].2.transmit);

    // Check stop loss order
    assert_eq!(submitted[2].0, 202);
    assert_eq!(submitted[2].2.action, Action::Sell);
    assert_eq!(submitted[2].2.order_type, "STP");
    assert_eq!(submitted[2].2.aux_price, Some(45.0));
    assert_eq!(submitted[2].2.parent_id, 200);
    assert!(submitted[2].2.transmit);
}

#[test]
fn test_submit_oca_orders() {
    let client = MockOrderClient::new();
    let contract1 = create_stock_contract("AAPL");
    let contract2 = create_stock_contract("MSFT");

    client.set_next_order_id(300);

    let order1 = OrderBuilder::new(&client, &contract1)
        .buy(100)
        .limit(50.0)
        .oca_group("TestOCA", 1)
        .build_order()
        .unwrap();

    let order2 = OrderBuilder::new(&client, &contract2)
        .buy(100)
        .limit(45.0)
        .oca_group("TestOCA", 1)
        .build_order()
        .unwrap();

    let order_ids = client.submit_oca_orders(vec![(contract1, order1), (contract2, order2)]).unwrap();

    assert_eq!(order_ids.len(), 2);
    assert_eq!(order_ids[0].value(), 300);
    assert_eq!(order_ids[1].value(), 301);

    // Verify OCA orders were submitted
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 2);
    assert_eq!(submitted[0].2.oca_group, "TestOCA");
    assert_eq!(submitted[1].2.oca_group, "TestOCA");
}

#[test]
fn test_order_submit_with_error() {
    let client = MockOrderClient::new();
    let contract = create_stock_contract("AAPL");

    // Set up an error response
    client.add_submit_order_response(Err(Error::Simple("Order rejected".to_string())));

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let result = builder.submit();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Order rejected"));
}

#[test]
fn test_analyze_no_response() {
    let client = MockOrderClient::new();
    let contract = create_stock_contract("AAPL");

    // Set up empty response (no order state)
    client.add_place_order_response(vec![]);

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let result = builder.analyze();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("What-if analysis did not return order state"));
}
