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
mod tests;
