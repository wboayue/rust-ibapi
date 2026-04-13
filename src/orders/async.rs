//! Asynchronous implementation of order management functionality

use time::OffsetDateTime;

use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::Subscription;
use crate::{Client, Error};

use super::common::{encoders, verify};
use super::*;

impl Client {
    /// Subscribes to order update events. Only one subscription can be active at a time.
    pub async fn order_update_stream(&self) -> Result<Subscription<OrderUpdate>, Error> {
        let internal_subscription = self.create_order_update_subscription().await?;
        Ok(Subscription::new_from_internal_simple::<OrderUpdate>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Submits an Order (fire-and-forget).
    pub async fn submit_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(order_id, contract, order)?;
        self.send_message(request).await?;

        Ok(())
    }

    /// Submits an Order with a subscription for updates.
    pub async fn place_order(&self, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error> {
        verify::verify_order(self, order, order_id)?;
        verify::verify_order_contract(self, contract, order_id)?;

        let request = encoders::encode_place_order(order_id, contract, order)?;
        let internal_subscription = self.send_order(order_id, request).await?;

        Ok(Subscription::new_from_internal_simple::<PlaceOrder>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Cancels an open [Order].
    pub async fn cancel_order(&self, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error> {
        if !manual_order_cancel_time.is_empty() {
            check_version(self.server_version(), Features::MANUAL_ORDER_TIME)?;
        }

        let request = encoders::encode_cancel_order(order_id, manual_order_cancel_time)?;
        let internal_subscription = self.send_order(order_id, request).await?;

        Ok(Subscription::new_from_internal_simple::<CancelOrder>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Cancels all open [Order]s.
    pub async fn global_cancel(&self) -> Result<(), Error> {
        check_version(self.server_version(), Features::REQ_GLOBAL_CANCEL)?;

        let message = encoders::encode_global_cancel()?;
        self.send_message(message).await?;

        Ok(())
    }

    /// Gets next valid order id
    pub async fn next_valid_order_id(&self) -> Result<i32, Error> {
        let message = encoders::encode_next_valid_order_id()?;

        let mut internal_subscription = self.send_shared_request(OutgoingMessages::RequestIds, message).await?;

        match internal_subscription.next().await {
            Some(Ok(message)) => {
                let order_id_index = 2;
                let next_order_id = message.peek_int(order_id_index)?;

                self.set_next_order_id(next_order_id);

                Ok(next_order_id)
            }
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no response from server".into())),
        }
    }

    /// Requests completed [Order]s.
    pub async fn completed_orders(&self, api_only: bool) -> Result<Subscription<Orders>, Error> {
        check_version(self.server_version(), Features::COMPLETED_ORDERS)?;

        let request = encoders::encode_completed_orders(api_only)?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestCompletedOrders, request).await?;
        Ok(Subscription::new_from_internal_simple::<Orders>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Requests all open orders placed by this specific API client.
    pub async fn open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_open_orders()?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple::<Orders>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Requests all *current* open orders in associated accounts.
    pub async fn all_open_orders(&self) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_all_open_orders()?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestAllOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple::<Orders>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Requests status updates about future orders placed from TWS.
    pub async fn auto_open_orders(&self, auto_bind: bool) -> Result<Subscription<Orders>, Error> {
        let request = encoders::encode_auto_open_orders(auto_bind)?;

        let internal_subscription = self.send_shared_request(OutgoingMessages::RequestAutoOpenOrders, request).await?;
        Ok(Subscription::new_from_internal_simple::<Orders>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Requests current day's executions matching the filter.
    pub async fn executions(&self, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error> {
        let request_id = self.next_request_id();
        let request = encoders::encode_executions(request_id, &filter)?;
        let internal_subscription = self.send_request(request_id, request).await?;
        Ok(Subscription::new_from_internal_simple::<Executions>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }

    /// Exercise an option contract.
    pub async fn exercise_options(
        &self,
        contract: &Contract,
        exercise_action: ExerciseAction,
        exercise_quantity: i32,
        account: &str,
        ovrd: bool,
        manual_order_time: Option<OffsetDateTime>,
    ) -> Result<Subscription<ExerciseOptions>, Error> {
        let order_id = self.next_order_id();
        let request = encoders::encode_exercise_options(order_id, contract, exercise_action, exercise_quantity, account, ovrd, manual_order_time)?;
        let internal_subscription = self.send_order(order_id, request).await?;
        Ok(Subscription::new_from_internal_simple::<ExerciseOptions>(
            internal_subscription,
            self.decoder_context(),
            self.message_bus.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::assert_proto_msg_id;
    use crate::contracts::{Contract, SecurityType};
    use crate::contracts::{Currency, Exchange, Symbol};
    use crate::messages::OutgoingMessages;
    use crate::stubs::MessageBusStub;
    use crate::{server_versions, Client};
    use std::sync::{Arc, RwLock};
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_place_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock OpenOrder response (based on real ESU5 order)
                "5|2|637533641|ES|FUT|20250919|0|?|50|CME|USD|ESU5|ES|BUY|1|LMT|5800.0|0.0|DAY||DU1234567||0||100|2126726143|0|0|0||2126726143.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Submitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|5801.0|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||||||".to_string(),
                // Mock OrderStatus response
                "3|1|Submitted|0|1|0|2126726143|0|0|100||0|".to_string(),
                // Mock ExecutionData response (no version field for server >= 136)
                "11|1|1|637533641|ES|FUT|20250919|0.0||50|CME|USD|ESU5|ES|0001f4e5.58bbad52.01.01|20250708 02:35:00 America/New_York|DU1234567|CME|BOT|1.0|5800.0|2126726143|100|0|1.0|5800.0|||0.0||1|".to_string(),
                // Mock CommissionReport response (with version field)
                "59|1|0001f4e5.58bbad52.01.01|2.25|USD|0.0|0.0||".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let contract = Contract {
            symbol: Symbol::from("ES"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("CME"),
            currency: Currency::from("USD"),
            local_symbol: "ESU5".to_string(),
            ..Default::default()
        };
        let mut order = order_builder::limit_order(Action::Buy, 1.0, 5800.0);
        order.order_id = 1;

        let mut subscription = client.place_order(1, &contract, &order).await.expect("failed to place order");

        // Test OpenOrder response
        let open_order = subscription.next().await;
        assert!(
            matches!(open_order, Some(Ok(PlaceOrder::OpenOrder(_)))),
            "Expected PlaceOrder::OpenOrder, got {:?}",
            open_order
        );

        // Test OrderStatus response
        let order_status = subscription.next().await;
        assert!(
            matches!(order_status, Some(Ok(PlaceOrder::OrderStatus(_)))),
            "Expected PlaceOrder::OrderStatus, got {:?}",
            order_status
        );

        // Test ExecutionData response
        let execution_data = subscription.next().await;
        assert!(
            matches!(execution_data, Some(Ok(PlaceOrder::ExecutionData(_)))),
            "Expected PlaceOrder::ExecutionData, got {:?}",
            execution_data
        );

        // Test CommissionReport response
        let commission_report = subscription.next().await;
        assert!(
            matches!(commission_report, Some(Ok(PlaceOrder::CommissionReport(_)))),
            "Expected PlaceOrder::CommissionReport, got {:?}",
            commission_report
        );

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        // The exact format depends on encode_place_order implementation
    }

    #[tokio::test]
    async fn test_cancel_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock OrderStatus response for cancelled order
                "3|1|Cancelled|0|1|0|2126726143|0|0|100||0|".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let mut subscription = client.cancel_order(1, "").await.expect("failed to cancel order");

        let cancel_response = subscription.next().await;
        assert!(
            matches!(cancel_response, Some(Ok(CancelOrder::OrderStatus(_)))),
            "Expected CancelOrder::OrderStatus, got {:?}",
            cancel_response
        );

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
    }

    #[tokio::test]
    async fn test_open_orders() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock OpenOrder response (simplified ES order)
                "5|2|637533641|ES|FUT|20250919|0|?|50|CME|USD|ESU5|ES|BUY|1|LMT|5800.0|0.0|DAY||DU1234567||0||100|2126726143|0|0|0||2126726143.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Submitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|5801.0|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||||||".to_string(),
                // Mock OrderStatus response
                "3|1|Submitted|0|1|0|2126726143|0|0|100||0|".to_string(),
                // Mock OpenOrderEnd
                "53|1|".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let mut subscription = client.open_orders().await.expect("failed to get open orders");

        // Test OrderData response
        let order_data = subscription.next().await;
        assert!(
            matches!(order_data, Some(Ok(Orders::OrderData(_)))),
            "Expected Orders::OrderData, got {:?}",
            order_data
        );

        // Test OrderStatus response
        let order_status = subscription.next().await;
        assert!(
            matches!(order_status, Some(Ok(Orders::OrderStatus(_)))),
            "Expected Orders::OrderStatus, got {:?}",
            order_status
        );

        // Test end of stream
        let end_response = subscription.next().await;
        assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestOpenOrders);
    }

    #[tokio::test]
    async fn test_completed_orders() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock CompletedOrder response (based on real completed ES order)
                "101|637533641|ES|FUT|20250919|0|?|50|CME|USD|ESU5|ES|BUY|1|LMT|5800.0|0.0|DAY||DU1236109||0||2126726143|0|0|0|||||||||||0||-1||||||2147483647|0|0||3|0||0|None||0|0|0||0|0||||0|0|0|2147483647|2147483647||||IB|0|0||0|Cancelled|0|0|0|5801.0|1.7976931348623157E308|0|1|0||0|2147483647|0|Not an insider or substantial shareholder|0|0|9223372036854775807|20250708 02:34:46 America/New_York|Cancelled by Trader||||||".to_string(),
                // Mock CompletedOrdersEnd
                "83||".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::COMPLETED_ORDERS);

        let mut subscription = client.completed_orders(true).await.expect("failed to get completed orders");

        // Test CompletedOrder response
        let completed_order = subscription.next().await;
        assert!(
            matches!(completed_order, Some(Ok(Orders::OrderData(_)))),
            "Expected Orders::OrderData, got {:?}",
            completed_order
        );

        // Test end of stream
        let end_response = subscription.next().await;
        assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestCompletedOrders);
    }

    #[tokio::test]
    async fn test_executions() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock ExecutionData response (no version field for server >= 136)
                "11|9000|1|637533641|ES|FUT|20250919|0.0||50|CME|USD|ESU5|ES|0001f4e5.58bbad52.01.01|20250708 02:35:00 America/New_York|DU1234567|CME|BOT|1.0|5800.0|2126726143|100|0|1.0|5800.0|||0.0||1|".to_string(),
                // Mock CommissionReport response (with version field)
                "59|1|0001f4e5.58bbad52.01.01|2.25|USD|0.0|0.0||".to_string(),
                // Mock ExecutionDataEnd
                "55|1|9000|".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let filter = ExecutionFilter::default();
        let mut subscription = client.executions(filter).await.expect("failed to get executions");

        // Test ExecutionData response
        let execution_data = subscription.next().await;
        assert!(
            matches!(execution_data, Some(Ok(Executions::ExecutionData(_)))),
            "Expected Executions::ExecutionData, got {:?}",
            execution_data
        );

        // Test CommissionReport response
        let commission_report = subscription.next().await;
        assert!(
            matches!(commission_report, Some(Ok(Executions::CommissionReport(_)))),
            "Expected Executions::CommissionReport, got {:?}",
            commission_report
        );

        // Test end of stream
        let end_response = subscription.next().await;
        assert!(end_response.is_none(), "Expected None (end of stream), got {:?}", end_response);

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        // Request format depends on encode_executions implementation
    }

    #[tokio::test]
    async fn test_submit_order() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let contract = Contract {
            symbol: Symbol::from("ES"),
            security_type: SecurityType::Future,
            exchange: Exchange::from("CME"),
            currency: Currency::from("USD"),
            local_symbol: "ESU5".to_string(),
            ..Default::default()
        };
        let mut order = order_builder::limit_order(Action::Buy, 1.0, 5800.0);
        order.order_id = 2;

        client.submit_order(2, &contract, &order).await.expect("failed to submit order");

        // Check request message was sent
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
    }

    #[tokio::test]
    async fn test_exercise_options() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock OpenOrder response for exercised option (adapted ES option)
                "5|2|637533642|ES|FOP|20250919|5800|C|50|CME|USD|ESU5C5800|ES|BUY|1|MKT|0.0|0.0|DAY||DU1234567||0||100|2126726144|0|0|0||2126726144.0/DU1234567/100||||||||||0||-1|0||||||2147483647|0|0|0||3|0|0||0|0||0|None||0||||?|0|0||0|0||||||0|0|0|2147483647|2147483647|||0||IB|0|0||0|0|Submitted|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308||||||0|0|0|None|1.7976931348623157E308|0.0|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|1.7976931348623157E308|0||||0|1|0|0|0|||0||||||".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let contract = Contract {
            symbol: Symbol::from("ES"),
            security_type: SecurityType::FuturesOption,
            exchange: Exchange::from("CME"),
            currency: Currency::from("USD"),
            last_trade_date_or_contract_month: "20250919".to_string(),
            strike: 5800.0,
            right: "C".to_string(),
            ..Default::default()
        };

        let mut subscription = client
            .exercise_options(&contract, ExerciseAction::Exercise, 1, "", false, None)
            .await
            .expect("failed to exercise options");

        let exercise_response = subscription.next().await;
        assert!(
            matches!(exercise_response, Some(Ok(ExerciseOptions::OpenOrder(_)))),
            "Expected ExerciseOptions::OpenOrder, got {:?}",
            exercise_response
        );

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
    }

    #[tokio::test]
    async fn test_next_valid_order_id() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["4|1|123|".to_string()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        // Check initial order ID
        let initial_order_id = client.next_order_id();

        let order_id = client.next_valid_order_id().await.expect("failed to get next valid order id");

        assert_eq!(order_id, 123, "Expected order ID 123");

        // Verify that the client's order ID was updated
        assert_eq!(client.next_order_id(), 123, "Client's order ID should be updated to 123");
        assert_ne!(client.next_order_id(), initial_order_id, "Client's order ID should have changed");

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestIds);
    }

    #[tokio::test]
    async fn test_order_update_stream() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                // Mock OrderStatus response - using same data as from test_place_order
                "3|100|Submitted|0|1|0|2126726143|0|0|100||0|".to_string(),
                // Mock ExecutionData response  
                "11|1|1|637533641|ES|FUT|20250919|0.0||50|CME|USD|ESU5|ES|0001f4e5.58bbad52.01.01|20250708 02:35:00 America/New_York|DU1234567|CME|BOT|1.0|5800.0|2126726143|100|0|1.0|5800.0|||0.0||1|".to_string(),
                // Mock CommissionReport response
                "59|1|0001f4e5.58bbad52.01.01|2.25|USD|0.0|0.0||".to_string(),
            ],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

        let mut stream = client.order_update_stream().await.unwrap();

        // Test that we can receive OrderStatus
        let update = stream.next().await.unwrap().unwrap();
        assert!(matches!(update, OrderUpdate::OrderStatus(_)));

        // Test that we can receive ExecutionData
        let update = stream.next().await.unwrap().unwrap();
        assert!(matches!(update, OrderUpdate::ExecutionData(_)));

        // Test that we can receive CommissionReport
        let update = stream.next().await.unwrap().unwrap();
        assert!(matches!(update, OrderUpdate::CommissionReport(_)));
    }

    #[tokio::test]
    async fn test_order_update_stream_already_subscribed() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let stream1 = client.order_update_stream().await;
        assert!(stream1.is_ok(), "failed to create first order update stream");

        let stream2 = client.order_update_stream().await;
        assert!(stream2.is_err(), "second order update stream should fail with AlreadySubscribed");
        assert!(
            matches!(stream2.err().unwrap(), Error::AlreadySubscribed),
            "expected AlreadySubscribed error"
        );
    }

    #[tokio::test]
    async fn test_order_update_stream_drop_releases_subscription() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });
        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let stream1 = client.order_update_stream().await.expect("failed to create initial order update stream");
        drop(stream1);

        // Allow cleanup task to process the drop signal
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        client.order_update_stream().await.expect("should be re-subscribable after drop");
    }

    #[tokio::test]
    async fn test_global_cancel() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_GLOBAL_CANCEL);

        client.global_cancel().await.expect("failed to send global cancel");

        // Check request message
        let request_messages = message_bus.request_messages.read().unwrap();
        assert_eq!(request_messages.len(), 1, "Expected one request message");
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestGlobalCancel);
    }
}
