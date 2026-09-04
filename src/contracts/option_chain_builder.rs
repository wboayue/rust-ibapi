//! Fluent builder for the option-chain request (TWS `reqSecDefOptParams`).
//!
//! Same shape as the WSH and historical-data builders: generic over `'a` + client
//! type, `mut self`-returning setters, per-feature terminal `impl` blocks. See
//! [`WshEventFilterBuilder`](crate::wsh::WshEventFilterBuilder) for the sibling
//! with the same single terminal.
//!
//! `option_chain` took four positional arguments. `exchange` was the one with a
//! default — `""` for all exchanges — and both it and `contract_id` were being
//! passed as sentinels (`"SMART"`, `0`) by examples in this tree that TWS answers
//! with an empty chain or a rejection. See
//! [param budget](../../docs/rules/style/param-budget.md).

use crate::contracts::{OptionChain, SecurityType};
use crate::Error;

/// Builder for an underlying's option chain: one [`OptionChain`] per exchange the
/// options trade on.
#[must_use = "OptionChainBuilder does nothing until you call .subscribe()"]
pub struct OptionChainBuilder<'a, C> {
    client: &'a C,
    symbol: &'a str,
    security_type: SecurityType,
    contract_id: i32,
    exchange: Option<&'a str>,
}

impl<'a, C> OptionChainBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, symbol: &'a str, security_type: SecurityType, contract_id: i32) -> Self {
        Self {
            client,
            symbol,
            security_type,
            contract_id,
            exchange: None,
        }
    }

    /// Restrict the chain to options trading on one exchange.
    ///
    /// This is TWS's `futFopExchange`: it selects the exchange for **futures
    /// options**. For a stock underlying leave it unset — TWS then returns one
    /// [`OptionChain`] per listing exchange (twenty for AAPL), whereas naming any
    /// of them, `SMART` included, returns an empty chain.
    pub fn exchange(mut self, exchange: &'a str) -> Self {
        self.exchange = Some(exchange);
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> OptionChainBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return a subscription yielding one [`OptionChain`]
    /// per exchange. The subscription ends when TWS has sent every exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::SecurityType;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// // Every exchange listing AAPL options (contract id 265598):
    /// let subscription = client
    ///     .option_chain("AAPL", SecurityType::Stock, 265598)
    ///     .subscribe()
    ///     .expect("request option chain failed");
    ///
    /// for chain in subscription.iter_data() {
    ///     let chain = chain.expect("decode error");
    ///     println!("{}: {} expirations, {} strikes", chain.exchange, chain.expirations.len(), chain.strikes.len());
    /// }
    ///
    /// // Futures options on one exchange:
    /// let subscription = client
    ///     .option_chain("ES", SecurityType::Future, 495512563)
    ///     .exchange("CME")
    ///     .subscribe()
    ///     .expect("request option chain failed");
    /// ```
    pub fn subscribe(self) -> Result<crate::subscriptions::sync::Subscription<OptionChain>, Error> {
        crate::contracts::sync::option_chain(self.client, self.symbol, self.exchange, self.security_type, self.contract_id)
    }
}

#[cfg(feature = "async")]
impl<'a> OptionChainBuilder<'a, crate::client::r#async::Client> {
    /// Submit the request and return a subscription yielding one [`OptionChain`]
    /// per exchange. The subscription ends when TWS has sent every exchange.
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
    ///     // Every exchange listing AAPL options (contract id 265598):
    ///     let subscription = client
    ///         .option_chain("AAPL", SecurityType::Stock, 265598)
    ///         .subscribe()
    ///         .await
    ///         .expect("request option chain failed");
    ///
    ///     let mut chains = subscription.filter_data();
    ///     while let Some(chain) = chains.next().await {
    ///         let chain = chain.expect("decode error");
    ///         println!("{}: {} expirations, {} strikes", chain.exchange, chain.expirations.len(), chain.strikes.len());
    ///     }
    ///
    ///     // Futures options on one exchange:
    ///     let subscription = client
    ///         .option_chain("ES", SecurityType::Future, 495512563)
    ///         .exchange("CME")
    ///         .subscribe()
    ///         .await
    ///         .expect("request option chain failed");
    /// }
    /// ```
    pub async fn subscribe(self) -> Result<crate::subscriptions::r#async::Subscription<OptionChain>, Error> {
        crate::contracts::r#async::option_chain(self.client, self.symbol, self.exchange, self.security_type, self.contract_id).await
    }
}
