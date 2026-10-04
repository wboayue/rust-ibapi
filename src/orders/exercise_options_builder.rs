//! Fluent builder for exercising or lapsing options (TWS `exerciseOptions`).
//!
//! Same shape as [`OptionChainBuilder`](crate::contracts::OptionChainBuilder): generic over
//! `'a` + client type, `mut self`-returning setters, per-feature terminal `impl` blocks.
//!
//! `exercise_options` took six positional arguments, two of them defaultable (`ovrd: bool`,
//! `manual_order_time: Option<_>`) and one a bare `bool`. See
//! [param budget](../../docs/rules/style/param-budget.md).

use time::OffsetDateTime;

use super::builder::ValidationError;
use super::common::encoders;
use super::ExerciseAction;
use crate::contracts::Contract;
use crate::orders::OrderId;
use crate::Error;

/// Builder for an option exercise or lapse request. Start it with `client.exercise_options(&contract)`,
/// pick [`exercise`](Self::exercise) or [`lapse`](Self::lapse), and [`submit`](Self::submit) it.
#[must_use = "ExerciseOptionsBuilder does nothing until you call .submit()"]
pub struct ExerciseOptionsBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    action: Option<(ExerciseAction, i32)>,
    account: Option<String>,
    override_natural_action: bool,
    manual_order_time: Option<OffsetDateTime>,
}

impl<'a, C> ExerciseOptionsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract) -> Self {
        Self {
            client,
            contract,
            action: None,
            account: None,
            override_natural_action: false,
            manual_order_time: None,
        }
    }

    /// Exercise `quantity` contracts.
    pub fn exercise(mut self, quantity: i32) -> Self {
        self.action = Some((ExerciseAction::Exercise, quantity));
        self
    }

    /// Let `quantity` contracts lapse.
    pub fn lapse(mut self, quantity: i32) -> Self {
        self.action = Some((ExerciseAction::Lapse, quantity));
        self
    }

    /// Account holding the options. If unset, no account is sent; on a single-account login TWS
    /// then uses the logged-in account.
    pub fn account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Override the system's natural action. For example, exercise an option that is out of the
    /// money, which by its natural action would not be exercised.
    pub fn override_natural_action(mut self) -> Self {
        self.override_natural_action = true;
        self
    }

    /// Time at which the options should be exercised. Defaults to now. Requires TWS API 10.26 or
    /// higher.
    pub fn manual_order_time(mut self, time: OffsetDateTime) -> Self {
        self.manual_order_time = Some(time);
        self
    }

    /// Validate the request and encode it under the next order id. Validation runs first, so a
    /// rejected request doesn't use up an order id.
    fn encode(&self, next_order_id: impl FnOnce() -> i32) -> Result<(OrderId, Vec<u8>), Error> {
        let (action, quantity) = self.action.ok_or(ValidationError::MissingRequiredField("exercise or lapse"))?;
        if quantity <= 0 {
            return Err(ValidationError::InvalidQuantity(f64::from(quantity)).into());
        }

        let order_id = OrderId::from(next_order_id()).checked()?;
        let request = encoders::encode_exercise_options(
            order_id.value(),
            self.contract,
            action,
            quantity,
            self.account.as_deref().unwrap_or_default(),
            self.override_natural_action,
            self.manual_order_time,
        )?;
        Ok((order_id, request))
    }
}

#[cfg(feature = "sync")]
impl ExerciseOptionsBuilder<'_, crate::client::sync::Client> {
    /// Send the request and return a subscription yielding its order status, open order and
    /// execution updates.
    ///
    /// Note: a TWS setting decides whether an exercise request must be finalized before it is sent.
    ///
    /// # Errors
    /// [`Error::InvalidArgument`] if neither [`exercise`](Self::exercise) nor [`lapse`](Self::lapse)
    /// was called, or the quantity is not positive; [`Error::OrderIdInRequestRange`] if the client's
    /// next order id has reached 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::{Contract, OptionRight};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::option("AAPL", "20251219", 150.0, OptionRight::Call);
    /// let subscription = client
    ///     .exercise_options(&contract)
    ///     .exercise(1)
    ///     .account("DU000001")
    ///     .submit()
    ///     .expect("exercise_options failed");
    ///
    /// // Consume the subscription so execution updates and commission reports surface.
    /// for event in subscription.iter_data() {
    ///     match event {
    ///         Ok(item) => println!("exercise event: {item:?}"),
    ///         Err(e) => { eprintln!("exercise err: {e:?}"); break; }
    ///     }
    /// }
    /// ```
    pub fn submit(self) -> Result<crate::subscriptions::sync::Subscription<super::ExerciseOptions>, Error> {
        let (order_id, request) = self.encode(|| self.client.next_order_id())?;
        let subscription = self.client.send_order(order_id, request)?;

        Ok(crate::subscriptions::sync::Subscription::new(
            std::sync::Arc::clone(&self.client.message_bus),
            subscription,
            self.client.decoder_context(),
        ))
    }
}

#[cfg(feature = "async")]
impl ExerciseOptionsBuilder<'_, crate::client::r#async::Client> {
    /// Send the request and return a subscription yielding its order status, open order and
    /// execution updates.
    ///
    /// Note: a TWS setting decides whether an exercise request must be finalized before it is sent.
    ///
    /// # Errors
    /// [`Error::InvalidArgument`] if neither [`exercise`](Self::exercise) nor [`lapse`](Self::lapse)
    /// was called, or the quantity is not positive; [`Error::OrderIdInRequestRange`] if the client's
    /// next order id has reached 1,500,000,000 (reserved for request ids).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::option("AAPL", "20251219", 150.0, OptionRight::Call);
    ///     let subscription = client
    ///         .exercise_options(&contract)
    ///         .exercise(1)
    ///         .account("DU000001")
    ///         .submit()
    ///         .await
    ///         .expect("exercise_options failed");
    ///
    ///     // Consume the subscription so execution updates and commission reports surface.
    ///     let mut events = subscription.filter_data();
    ///     while let Some(event) = events.next().await {
    ///         match event {
    ///             Ok(item) => println!("exercise event: {item:?}"),
    ///             Err(e) => { eprintln!("exercise err: {e:?}"); break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn submit(self) -> Result<crate::subscriptions::Subscription<super::ExerciseOptions>, Error> {
        let (order_id, request) = self.encode(|| self.client.next_order_id())?;
        let internal_subscription = self.client.send_order(order_id, request).await?;

        Ok(crate::subscriptions::Subscription::new_from_internal_simple(
            internal_subscription,
            self.client.message_bus.clone(),
            self.client.decoder_context(),
        ))
    }
}

#[cfg(test)]
#[path = "exercise_options_builder_tests.rs"]
mod tests;
