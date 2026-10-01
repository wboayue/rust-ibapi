//! Fluent builder for a streamed contract-details request (TWS `reqContractDetails`).
//!
//! Same shape as [`OptionChainBuilder`](crate::contracts::OptionChainBuilder):
//! generic over `'a` + client type, per-feature terminal `impl` blocks. The
//! request id is allocated when the builder is made, so a caller can record it
//! before anything is written; `subscribe` sends once and never retries.

use crate::contracts::{Contract, ContractDetails};
use crate::Error;

/// Builder for a contract-details request that yields one [`ContractDetails`]
/// per matching contract.
///
/// Made by `Client::contract_details_stream`. Unlike `Client::contract_details`,
/// which collects every row, the subscription can be read incrementally and
/// dropped early, and its request id is known before the request is sent.
///
/// Dropping it before the end sends TWS's native cancel (server 215+). The
/// cancel is not guaranteed to stop delivery: observed live, TWS sends a
/// result it has already prepared in full anyway. Rows arriving after the
/// drop are discarded.
#[must_use = "ContractDetailsBuilder does nothing until you call .subscribe()"]
pub struct ContractDetailsBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    request_id: i32,
}

impl<'a, C> ContractDetailsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, request_id: i32) -> Self {
        Self {
            client,
            contract,
            request_id,
        }
    }

    /// The request id `subscribe` will send. Allocated when the builder was
    /// made; nothing has been written yet. A dropped builder skips the id.
    pub fn request_id(&self) -> i32 {
        self.request_id
    }
}

#[cfg(feature = "sync")]
impl<'a> ContractDetailsBuilder<'a, crate::client::sync::Client> {
    /// Validate the contract and send the request once, returning a
    /// subscription that yields one [`ContractDetails`] per matching contract
    /// and ends when TWS has sent them all.
    ///
    /// A validation error is returned before anything is written. There is no
    /// retry: after a connection reset, build a new request (new id).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::subscriptions::SubscriptionItem;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let request = client.contract_details_stream(&contract);
    /// println!("request id: {}", request.request_id());
    ///
    /// let subscription = request.subscribe().expect("request failed");
    /// let mut rows = 0;
    /// while let Some(item) = subscription.next() {
    ///     match item {
    ///         Ok(SubscriptionItem::Data(details)) => {
    ///             println!("{} on {}", details.contract.symbol, details.contract.exchange);
    ///             rows += 1;
    ///             if rows == 10 {
    ///                 break; // dropping the subscription sends the native cancel
    ///             }
    ///         }
    ///         Ok(SubscriptionItem::Notice(notice)) => eprintln!("notice: {notice}"),
    ///         Err(e) => {
    ///             eprintln!("error: {e}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<ContractDetails>, Error> {
        crate::contracts::sync::contract_details_stream(self.client, self.contract, self.request_id)
    }
}

#[cfg(feature = "async")]
impl<'a> ContractDetailsBuilder<'a, crate::client::r#async::Client> {
    /// Validate the contract and send the request once, returning a
    /// subscription that yields one [`ContractDetails`] per matching contract
    /// and ends when TWS has sent them all.
    ///
    /// A validation error is returned before anything is written. There is no
    /// retry: after a connection reset, build a new request (new id).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL").build();
    ///     let request = client.contract_details_stream(&contract);
    ///     println!("request id: {}", request.request_id());
    ///
    ///     let mut subscription = request.subscribe().await.expect("request failed");
    ///     let mut rows = 0;
    ///     while let Some(item) = subscription.next().await {
    ///         match item {
    ///             Ok(SubscriptionItem::Data(details)) => {
    ///                 println!("{} on {}", details.contract.symbol, details.contract.exchange);
    ///                 rows += 1;
    ///                 if rows == 10 {
    ///                     break; // dropping the subscription sends the native cancel
    ///                 }
    ///             }
    ///             Ok(SubscriptionItem::Notice(notice)) => eprintln!("notice: {notice}"),
    ///             Err(e) => {
    ///                 eprintln!("error: {e}");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<ContractDetails>, Error> {
        crate::contracts::r#async::contract_details_stream(self.client, self.contract, self.request_id).await
    }
}
