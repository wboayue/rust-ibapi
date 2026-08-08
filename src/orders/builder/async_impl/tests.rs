use crate::contracts::{Contract, Currency, Exchange, Symbol};
use crate::errors::Error;
use crate::orders::builder::tests::async_mock_client::mock::AsyncMockClient;
use crate::orders::{Action, BracketOrderBuilder, BracketOrderIds, Order, OrderBuilder, OrderId, TimeInForce};

fn create_stock_contract(symbol: &str) -> Contract {
    Contract {
        symbol: Symbol::from(symbol),
        security_type: crate::contracts::SecurityType::Stock,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    }
}

// Implement the async methods for MockClient
impl<'a> OrderBuilder<'a, AsyncMockClient> {
    /// Build the order and return it without submitting
    pub fn build_order(self) -> Result<Order, Error> {
        self.build().map_err(Into::into)
    }

    /// Submit the order using async mock client
    pub async fn submit(self) -> Result<OrderId, Error> {
        let client = self.client;
        let contract = self.contract;
        let order_id = client.next_order_id();
        let order = self.build()?;
        client.submit_order(order_id, contract, &order).await?;
        Ok(OrderId::new(order_id))
    }
}

// Implement submit_all for bracket orders
impl<'a> BracketOrderBuilder<'a, AsyncMockClient> {
    /// Submit the bracket order trio asynchronously and return the generated identifiers.
    pub async fn submit_all(self) -> Result<BracketOrderIds, Error> {
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

            client.submit_order(order_id, contract, &order).await?;
        }

        Ok(BracketOrderIds::new(order_ids[0], order_ids[1], order_ids[2]))
    }
}

#[tokio::test]
async fn test_async_order_submit() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    client.set_next_order_id(100);

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let order_id = builder.submit().await.unwrap();
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

#[tokio::test]
async fn test_async_bracket_order_submit_all() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    client.set_next_order_id(200);

    let bracket_builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let bracket_ids = bracket_builder.submit_all().await.unwrap();
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

#[tokio::test]
async fn test_async_bracket_order_propagates_tif() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    client.set_next_order_id(200);

    let bracket_builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .good_till_cancel()
        .bracket()
        .entry_limit(50.0)
        .take_profit(55.0)
        .stop_loss(45.0);

    let _bracket_ids = bracket_builder.submit_all().await.unwrap();

    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 3);

    // All three orders should have GTC
    assert_eq!(submitted[0].2.tif, TimeInForce::GoodTilCanceled);
    assert_eq!(submitted[1].2.tif, TimeInForce::GoodTilCanceled);
    assert_eq!(submitted[2].2.tif, TimeInForce::GoodTilCanceled);
}

#[tokio::test]
async fn test_async_submit_oca_orders() {
    let client = AsyncMockClient::new();
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

    let order_ids = client.submit_oca_orders(vec![(contract1, order1), (contract2, order2)]).await.unwrap();

    assert_eq!(order_ids.len(), 2);
    assert_eq!(order_ids[0].value(), 300);
    assert_eq!(order_ids[1].value(), 301);

    // Verify OCA orders were submitted
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted.len(), 2);
    assert_eq!(submitted[0].2.oca_group, "TestOCA");
    assert_eq!(submitted[1].2.oca_group, "TestOCA");
}

#[tokio::test]
async fn test_async_order_submit_with_error() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    // Set up an error response
    client.add_submit_order_response(Err(Error::Simple("Order rejected".to_string())));

    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(50.00);

    let result = builder.submit().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Order rejected"));
}

#[tokio::test]
async fn test_async_complex_order_types() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    // Test trailing stop order
    let builder = OrderBuilder::new(&client, &contract).sell(100).trailing_stop(5.0, 95.0);

    let _order_id = builder.submit().await.unwrap();
    let submitted = client.get_submitted_orders();
    assert_eq!(submitted[0].2.order_type, "TRAIL");
    assert_eq!(submitted[0].2.trailing_percent, Some(5.0));
    assert_eq!(submitted[0].2.trail_stop_price, Some(95.0));

    // Test algo order
    let builder = OrderBuilder::new(&client, &contract)
        .buy(100)
        .limit(50.00)
        .algo("VWAP")
        .algo_param("startTime", "09:30:00")
        .algo_param("endTime", "16:00:00");

    builder.submit().await.unwrap();
    let submitted = client.get_submitted_orders();
    let last = submitted.last().unwrap();
    assert_eq!(last.2.algo_strategy, "VWAP");
    assert_eq!(last.2.algo_params.len(), 2);
}

#[tokio::test]
async fn test_async_order_validation() {
    let client = AsyncMockClient::new();
    let contract = create_stock_contract("AAPL");

    // Test invalid quantity
    let builder = OrderBuilder::new(&client, &contract).buy(-100).market();

    let result = builder.submit().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid quantity"));

    // Test invalid price (NaN)
    let builder = OrderBuilder::new(&client, &contract).buy(100).limit(f64::NAN);

    let result = builder.submit().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid price"));
}
