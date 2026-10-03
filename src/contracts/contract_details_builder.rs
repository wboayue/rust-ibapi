//! Fluent builder for a streamed contract-details request (TWS `reqContractDetails`).
//!
//! Same shape as [`OptionChainBuilder`](crate::contracts::OptionChainBuilder):
//! generic over `'a` + client type, per-feature terminal `impl` blocks. The
//! request id is allocated when the builder is made, so a caller can record it
//! before anything is written; `subscribe` sends once and never retries.

use crate::client::ids::RequestId;
use crate::contracts::{Contract, ContractDetails};
use crate::Error;

/// The largest [`ContractDetailsBuilder::buffer_limit`]. The async client
/// allocates its channel's slots up front: `limit + 1`, rounded up to a power
/// of two. At this maximum that is 65,536 slots, a few MiB.
pub const MAX_BUFFER_LIMIT: usize = 65_535;

/// Builder for a contract-details request that yields one [`ContractDetails`]
/// per matching contract.
///
/// Made by `Client::contract_details_stream`. Unlike `Client::contract_details`,
/// which collects every row, the subscription can be read incrementally and
/// dropped early, and its request id is known before the request is sent.
///
/// Dropping the subscription before TWS's end marker has been read sends the
/// native cancel (server 215+); dropping the builder sends nothing. Observed
/// live, TWS keeps sending after the cancel, so the cancel saves no gateway
/// work; rows arriving after the drop are discarded. To know when TWS is done
/// with the request, use `cancel_and_drain` instead of dropping.
#[must_use = "ContractDetailsBuilder does nothing until you call .subscribe()"]
pub struct ContractDetailsBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    request_id: RequestId,
    buffer_limit: Option<usize>,
}

impl<'a, C> ContractDetailsBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, request_id: RequestId) -> Self {
        Self {
            client,
            contract,
            request_id,
            buffer_limit: None,
        }
    }

    /// The request id `subscribe` will send. Allocated when the builder was
    /// made; nothing has been written yet. A dropped builder skips the id.
    pub fn request_id(&self) -> i32 {
        self.request_id.raw()
    }

    /// Fail the stream instead of queueing more than `limit` unread items.
    ///
    /// Items are rows and any TWS notices for the request. When `limit` are
    /// waiting to be read and another row or notice arrives, the subscription
    /// yields every queued item, then [`Error::BufferLimitExceeded`], then
    /// ends; anything TWS sends after that is discarded. TWS's end marker and
    /// errors always get through, so a result that fills the cap exactly still
    /// ends normally. A reader that keeps up never hits the cap, however many
    /// rows the query returns. Use it when the reader can stall (a slow sink,
    /// batching) and unbounded queueing is not acceptable.
    ///
    /// Without it, queueing is unbounded on the sync client, and on the async
    /// client capped by `ClientBuilder::channel_capacity` with the oldest rows
    /// dropped (reported as a lag notice).
    ///
    /// `limit` must be `1..=`[`MAX_BUFFER_LIMIT`]; otherwise `subscribe`
    /// returns [`Error::InvalidArgument`] without sending.
    ///
    /// On the async client only the original subscription's reads count. A
    /// clone's reads don't: if the original stops reading (or is dropped), the
    /// stream overflows after `limit` more items even when a clone keeps up.
    ///
    /// # Examples
    #[cfg_attr(
        feature = "sync",
        doc = r#"
```no_run
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::Error;

let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

let contract = Contract::stock("AAPL").build();
let subscription = client.contract_details_stream(&contract).buffer_limit(64).subscribe().expect("request failed");
for details in subscription.iter_data() {
    match details {
        Ok(details) => println!("{}", details.contract.contract_id),
        Err(Error::BufferLimitExceeded { limit }) => eprintln!("fell {limit} rows behind; stopping"),
        Err(e) => eprintln!("error: {e}"),
    }
}
```
"#
    )]
    #[cfg_attr(
        feature = "async",
        doc = r#"
```no_run
use ibapi::prelude::*;
use ibapi::Error;

#[tokio::main]
async fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    let contract = Contract::stock("AAPL").build();
    let subscription = client.contract_details_stream(&contract).buffer_limit(64).subscribe().await.expect("request failed");
    let mut details = subscription.filter_data();
    while let Some(details) = details.next().await {
        match details {
            Ok(details) => println!("{}", details.contract.contract_id),
            Err(Error::BufferLimitExceeded { limit }) => eprintln!("fell {limit} rows behind; stopping"),
            Err(e) => eprintln!("error: {e}"),
        }
    }
}
```
"#
    )]
    pub fn buffer_limit(mut self, limit: usize) -> Self {
        self.buffer_limit = Some(limit);
        self
    }
}

/// `limit` if it is a valid [`ContractDetailsBuilder::buffer_limit`].
pub(crate) fn validate_buffer_limit(limit: Option<usize>) -> Result<Option<usize>, Error> {
    match limit {
        Some(limit) if !(1..=MAX_BUFFER_LIMIT).contains(&limit) => Err(Error::InvalidArgument(format!(
            "buffer_limit must be 1..={MAX_BUFFER_LIMIT}, got {limit}"
        ))),
        limit => Ok(limit),
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
        crate::contracts::sync::contract_details_stream(self.client, self.contract, self.request_id, self.buffer_limit)
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
        crate::contracts::r#async::contract_details_stream(self.client, self.contract, self.request_id, self.buffer_limit).await
    }
}
