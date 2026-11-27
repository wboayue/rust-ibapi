//! Builders for the account subscriptions that take an optional account or model code:
//! `positions_multi`, `account_updates_multi`, `pnl` and `pnl_single`.
//!
//! Same shape as the option-chain and historical-news builders: generic over `'a` + client
//! type, `mut self`-returning setters, per-feature `subscribe` terminals. Setters take
//! `impl Into<AccountId>` / `impl Into<ModelCode>`, so an unset filter is no call rather than a
//! bare `None` that needs a type; see `docs/rules/style/id-parameters.md`.

use super::common::encoders;
use super::types::{AccountId, ModelCode};
use super::LedgerOnly;
use crate::contracts::ContractId;
use crate::Error;

#[cfg(test)]
#[path = "builders_tests.rs"]
mod tests;

/// Builder for a [`PositionUpdateMulti`](super::PositionUpdateMulti) subscription, from
/// `Client::positions_multi`.
///
/// Without a filter the request goes out with no account and no model code;
/// [`account`](Self::account) and [`model_code`](Self::model_code) narrow it.
#[must_use = "PositionsMultiBuilder does nothing until you call .subscribe()"]
pub struct PositionsMultiBuilder<'a, C> {
    client: &'a C,
    account: Option<AccountId>,
    model_code: Option<ModelCode>,
}

impl<'a, C> PositionsMultiBuilder<'a, C> {
    pub(crate) fn new(client: &'a C) -> Self {
        Self {
            client,
            account: None,
            model_code: None,
        }
    }

    /// Only this account's positions.
    pub fn account(mut self, account: impl Into<AccountId>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Only positions belonging to this model.
    pub fn model_code(mut self, model_code: impl Into<ModelCode>) -> Self {
        self.model_code = Some(model_code.into());
        self
    }

    fn encode(&self, request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_request_positions_multi(request_id, self.account.as_ref(), self.model_code.as_ref())
    }
}

/// Builder for an [`AccountUpdateMulti`](super::AccountUpdateMulti) subscription, from
/// `Client::account_updates_multi`.
///
/// Without a filter the request goes out with no account and no model code;
/// [`account`](Self::account) and [`model_code`](Self::model_code) narrow it.
#[must_use = "AccountUpdatesMultiBuilder does nothing until you call .subscribe()"]
pub struct AccountUpdatesMultiBuilder<'a, C> {
    client: &'a C,
    account: Option<AccountId>,
    model_code: Option<ModelCode>,
    ledger_only: LedgerOnly,
}

impl<'a, C> AccountUpdatesMultiBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, ledger_only: LedgerOnly) -> Self {
        Self {
            client,
            account: None,
            model_code: None,
            ledger_only,
        }
    }

    /// Only this account's updates.
    pub fn account(mut self, account: impl Into<AccountId>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Only updates for this model.
    pub fn model_code(mut self, model_code: impl Into<ModelCode>) -> Self {
        self.model_code = Some(model_code.into());
        self
    }

    fn encode(&self, request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_request_account_updates_multi(request_id, self.account.as_ref(), self.model_code.as_ref(), self.ledger_only)
    }
}

/// Builder for an account's daily [`PnL`](super::PnL) subscription, from `Client::pnl`.
#[must_use = "PnLBuilder does nothing until you call .subscribe()"]
pub struct PnLBuilder<'a, C> {
    client: &'a C,
    account: AccountId,
    model_code: Option<ModelCode>,
}

impl<'a, C> PnLBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, account: impl Into<AccountId>) -> Self {
        Self {
            client,
            account: account.into(),
            model_code: None,
        }
    }

    /// PnL for this model only.
    pub fn model_code(mut self, model_code: impl Into<ModelCode>) -> Self {
        self.model_code = Some(model_code.into());
        self
    }

    fn encode(&self, request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_request_pnl(request_id, &self.account, self.model_code.as_ref())
    }
}

/// Builder for one position's daily [`PnLSingle`](super::PnLSingle) subscription, from
/// `Client::pnl_single`.
#[must_use = "PnLSingleBuilder does nothing until you call .subscribe()"]
pub struct PnLSingleBuilder<'a, C> {
    client: &'a C,
    account: AccountId,
    contract_id: ContractId,
    model_code: Option<ModelCode>,
}

impl<'a, C> PnLSingleBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, account: impl Into<AccountId>, contract_id: impl Into<ContractId>) -> Self {
        Self {
            client,
            account: account.into(),
            contract_id: contract_id.into(),
            model_code: None,
        }
    }

    /// The model the position is held in.
    pub fn model_code(mut self, model_code: impl Into<ModelCode>) -> Self {
        self.model_code = Some(model_code.into());
        self
    }

    fn encode(&self, request_id: i32) -> Result<Vec<u8>, Error> {
        encoders::encode_request_pnl_single(request_id, &self.account, self.contract_id, self.model_code.as_ref())
    }
}

#[cfg(feature = "sync")]
mod sync_impl {
    use super::*;
    use crate::accounts::{AccountUpdateMulti, PnL, PnLSingle, PositionUpdateMulti};
    use crate::client::blocking::{Client, ClientRequestBuilders, Subscription};

    impl PositionsMultiBuilder<'_, Client> {
        /// Sends the request. TWS sends the current positions, then
        /// [`PositionEnd`](PositionUpdateMulti::PositionEnd), then updates as positions change.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::accounts::PositionUpdateMulti;
        /// use ibapi::client::blocking::Client;
        ///
        /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
        /// let subscription = client.positions_multi().account("DU1234567").subscribe()?;
        /// for update in subscription.iter_data() {
        ///     match update? {
        ///         PositionUpdateMulti::Position(position) => println!("{position:?}"),
        ///         PositionUpdateMulti::PositionEnd => break,
        ///     }
        /// }
        /// # Ok::<(), ibapi::Error>(())
        /// ```
        pub fn subscribe(self) -> Result<Subscription<PositionUpdateMulti>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request)
        }
    }

    impl AccountUpdatesMultiBuilder<'_, Client> {
        /// Sends the request. TWS sends the current values, then
        /// [`End`](AccountUpdateMulti::End), then updates as values change.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
        /// use ibapi::client::blocking::Client;
        ///
        /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
        /// let subscription = client.account_updates_multi(LedgerOnly::No).account("DU1234567").subscribe()?;
        /// for update in subscription.iter_data() {
        ///     match update? {
        ///         AccountUpdateMulti::AccountMultiValue(value) => println!("{}: {} {}", value.key, value.value, value.currency),
        ///         AccountUpdateMulti::End => break,
        ///     }
        /// }
        /// # Ok::<(), ibapi::Error>(())
        /// ```
        pub fn subscribe(self) -> Result<Subscription<AccountUpdateMulti>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request)
        }
    }

    impl PnLBuilder<'_, Client> {
        /// Sends the request; TWS pushes [`PnL`] updates until the subscription is dropped.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::client::blocking::Client;
        ///
        /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
        /// let subscription = client.pnl("DU1234567").subscribe()?;
        /// for pnl in subscription.iter_data().take(5) {
        ///     let pnl = pnl?;
        ///     println!("daily {:?} unrealized {:?} realized {:?}", pnl.daily_pnl, pnl.unrealized_pnl, pnl.realized_pnl);
        /// }
        /// # Ok::<(), ibapi::Error>(())
        /// ```
        pub fn subscribe(self) -> Result<Subscription<PnL>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request)
        }
    }

    impl PnLSingleBuilder<'_, Client> {
        /// Sends the request; TWS pushes [`PnLSingle`] updates until the subscription is dropped.
        /// For a contract the account has never held, TWS sends nothing at all.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::client::blocking::Client;
        ///
        /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
        /// let subscription = client.pnl_single("DU1234567", 265598).subscribe()?; // AAPL
        /// for pnl in subscription.iter_data().take(5) {
        ///     let pnl = pnl?;
        ///     println!("position {} daily {:?} value {:?}", pnl.position, pnl.daily_pnl, pnl.value);
        /// }
        /// # Ok::<(), ibapi::Error>(())
        /// ```
        pub fn subscribe(self) -> Result<Subscription<PnLSingle>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request)
        }
    }
}

#[cfg(feature = "async")]
mod async_impl {
    use super::*;
    use crate::accounts::{AccountUpdateMulti, PnL, PnLSingle, PositionUpdateMulti};
    use crate::client::r#async::Client;
    use crate::client::ClientRequestBuilders;
    use crate::subscriptions::r#async::Subscription;

    impl PositionsMultiBuilder<'_, Client> {
        /// Sends the request. TWS sends the current positions, then
        /// [`PositionEnd`](PositionUpdateMulti::PositionEnd), then updates as positions change.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::accounts::PositionUpdateMulti;
        /// use ibapi::prelude::*;
        ///
        /// #[tokio::main]
        /// async fn main() -> Result<(), ibapi::Error> {
        ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
        ///     let mut updates = client.positions_multi().account("DU1234567").subscribe().await?.filter_data();
        ///     while let Some(update) = updates.next().await {
        ///         match update? {
        ///             PositionUpdateMulti::Position(position) => println!("{position:?}"),
        ///             PositionUpdateMulti::PositionEnd => break,
        ///         }
        ///     }
        ///     Ok(())
        /// }
        /// ```
        pub async fn subscribe(self) -> Result<Subscription<PositionUpdateMulti>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request).await
        }
    }

    impl AccountUpdatesMultiBuilder<'_, Client> {
        /// Sends the request. TWS sends the current values, then
        /// [`End`](AccountUpdateMulti::End), then updates as values change.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::accounts::{AccountUpdateMulti, LedgerOnly};
        /// use ibapi::prelude::*;
        ///
        /// #[tokio::main]
        /// async fn main() -> Result<(), ibapi::Error> {
        ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
        ///     let mut updates = client
        ///         .account_updates_multi(LedgerOnly::No)
        ///         .account("DU1234567")
        ///         .subscribe()
        ///         .await?
        ///         .filter_data();
        ///     while let Some(update) = updates.next().await {
        ///         match update? {
        ///             AccountUpdateMulti::AccountMultiValue(value) => println!("{}: {} {}", value.key, value.value, value.currency),
        ///             AccountUpdateMulti::End => break,
        ///         }
        ///     }
        ///     Ok(())
        /// }
        /// ```
        pub async fn subscribe(self) -> Result<Subscription<AccountUpdateMulti>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request).await
        }
    }

    impl PnLBuilder<'_, Client> {
        /// Sends the request; TWS pushes [`PnL`] updates until the subscription is dropped.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::prelude::*;
        ///
        /// #[tokio::main]
        /// async fn main() -> Result<(), ibapi::Error> {
        ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
        ///     let mut updates = client.pnl("DU1234567").subscribe().await?.filter_data().take(5);
        ///     while let Some(pnl) = updates.next().await {
        ///         let pnl = pnl?;
        ///         println!("daily {:?} unrealized {:?} realized {:?}", pnl.daily_pnl, pnl.unrealized_pnl, pnl.realized_pnl);
        ///     }
        ///     Ok(())
        /// }
        /// ```
        pub async fn subscribe(self) -> Result<Subscription<PnL>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request).await
        }
    }

    impl PnLSingleBuilder<'_, Client> {
        /// Sends the request; TWS pushes [`PnLSingle`] updates until the subscription is dropped.
        /// For a contract the account has never held, TWS sends nothing at all.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ibapi::prelude::*;
        ///
        /// #[tokio::main]
        /// async fn main() -> Result<(), ibapi::Error> {
        ///     let client = Client::connect("127.0.0.1:4002", 100).await?;
        ///     let mut updates = client.pnl_single("DU1234567", 265598).subscribe().await?.filter_data().take(5); // AAPL
        ///     while let Some(pnl) = updates.next().await {
        ///         let pnl = pnl?;
        ///         println!("position {} daily {:?} value {:?}", pnl.position, pnl.daily_pnl, pnl.value);
        ///     }
        ///     Ok(())
        /// }
        /// ```
        pub async fn subscribe(self) -> Result<Subscription<PnLSingle>, Error> {
            let builder = self.client.request();
            let request = self.encode(builder.request_id())?;
            builder.send(request).await
        }
    }
}
