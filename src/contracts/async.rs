//! Asynchronous implementation of contract management functionality

use super::common::{decoders, encoders, verify};
use super::*;
use crate::client::ids::RequestId;
use crate::client::ClientRequestBuilders;
use crate::common::request_helpers::{self, empty_on_end_of_stream, expect_proto};
use crate::messages::OutgoingMessages;
use crate::protocol::{check_version, Features};
use crate::subscriptions::{StreamDecoder, Subscription};
use crate::{Client, Error};

impl Client {
    /// Requests contract information.
    ///
    /// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains.
    ///
    /// Collects every row before returning. To read rows as they arrive, stop reading early, or know the
    /// request id up front, use [Client::contract_details_stream].
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let contract = Contract::stock("AAPL").build();
    ///     let details = client.contract_details(&contract).await.expect("request failed");
    ///
    ///     for detail in details {
    ///         println!("Contract: {} - Exchange: {}", detail.contract.symbol, detail.contract.exchange);
    ///     }
    /// }
    /// ```
    pub async fn contract_details(&self, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
        self.contract_details_stream(contract).subscribe().await?.collect_to_end().await
    }

    /// Build a contract-details request whose subscription yields one
    /// [ContractDetails] per matching contract.
    ///
    /// Use this over [Client::contract_details] to read rows as they arrive,
    /// stop reading early, or know the request id before anything is sent.
    /// Dropping the subscription before the end sends TWS's native cancel
    /// (server 215+); rows TWS sends after that are discarded.
    /// Terminal: [`ContractDetailsBuilder::subscribe`].
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts.
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
    ///     let request_id = request.request_id(); // known before anything is sent
    ///
    ///     let subscription = request.subscribe().await.expect("request failed");
    ///     let mut details = subscription.filter_data().take(5);
    ///     while let Some(details) = details.next().await {
    ///         match details {
    ///             Ok(details) => println!("[{request_id}] {} on {}", details.contract.symbol, details.contract.exchange),
    ///             Err(e) => {
    ///                 eprintln!("error: {e}");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn contract_details_stream<'a>(&'a self, contract: &'a Contract) -> ContractDetailsBuilder<'a, Self> {
        ContractDetailsBuilder::new(self, contract, self.mint_request_id())
    }

    /// Requests matching stock symbols.
    ///
    /// # Arguments
    /// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
    ///
    /// # Retries
    ///
    /// If the connection resets mid-request, this waits for the reconnect and
    /// sends the request again with a fresh request id, up to 3 times (4
    /// attempts in all). Other errors are not retried. A caller pacing requests
    /// against TWS limits should count each reconnect (see the notice stream) as
    /// a possible extra request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let symbols = client.matching_symbols("AAP").await.expect("request failed");
    ///     for symbol in symbols {
    ///         println!("{} - {} ({})", symbol.contract.symbol,
    ///                  symbol.contract.primary_exchange, symbol.contract.currency);
    ///     }
    /// }
    /// ```
    pub async fn matching_symbols(&self, pattern: &str) -> Result<Vec<ContractDescription>, Error> {
        check_version(self.server_version(), Features::REQ_MATCHING_SYMBOLS)?;

        request_helpers::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_request_matching_symbols(request_id, pattern),
            expect_proto(decoders::decode_symbol_samples_proto),
        )
        .await
        .or_else(empty_on_end_of_stream)
    }

    /// Requests details about a given market rule.
    ///
    /// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
    ///
    /// # Arguments
    /// * `market_rule_id` - The market rule ID to query
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let rule = client.market_rule(26).await.expect("request failed");
    ///     for increment in rule.price_increments {
    ///         println!("Above ${}: increment ${}", increment.low_edge, increment.increment);
    ///     }
    /// }
    /// ```
    pub async fn market_rule(&self, market_rule_id: i32) -> Result<MarketRule, Error> {
        check_version(self.server_version(), Features::MARKET_RULES)?;

        request_helpers::one_shot_shared(
            self,
            OutgoingMessages::RequestMarketRule,
            || encoders::encode_request_market_rule(market_rule_id),
            expect_proto(decoders::decode_market_rule_proto),
        )
        .await
    }

    /// Requests the underlying exchanges that contribute to a consolidated (BBO) feed.
    ///
    /// Given a BBO exchange code (an opaque per-session token, e.g. `"a6"`),
    /// returns the list of underlying exchanges with each entry's bit
    /// position, full exchange name, and single-letter abbreviation. Useful
    /// for decoding the `mdSize` / `mdMask` bitmaps on tick-by-tick and
    /// market-depth streams. The token is typically obtained from the
    /// `LAST_EXCHANGE` market-data tick (tick type 84).
    ///
    /// # Arguments
    /// * `bbo_exchange` - The BBO exchange token (e.g. `"a6"`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let components = client.smart_components("a6").await.expect("request failed");
    ///     for component in &components {
    ///         println!("bit {}: {} ({})", component.bit_number, component.exchange, component.exchange_letter);
    ///     }
    /// }
    /// ```
    pub async fn smart_components(&self, bbo_exchange: &str) -> Result<Vec<SmartComponent>, Error> {
        check_version(self.server_version(), Features::SMART_COMPONENTS)?;

        request_helpers::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_request_smart_components(request_id, bbo_exchange),
            expect_proto(decoders::decode_smart_components_proto),
        )
        .await
    }

    /// Calculates an option's price based on the provided volatility and its underlying's price.
    ///
    /// # Arguments
    /// * `contract`   - The [Contract] object representing the option for which the calculation is being requested.
    /// * `volatility` - Hypothetical volatility.
    /// * `underlying_price` - Hypothetical option's underlying price.
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
    ///     let calculation = client
    ///         .calculate_option_price(&contract, 100.0, 235.0)
    ///         .await
    ///         .expect("request failed");
    ///     println!("calculation: {calculation:?}");
    /// }
    /// ```
    pub async fn calculate_option_price(&self, contract: &Contract, volatility: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        check_version(self.server_version(), Features::REQ_CALC_OPTION_PRICE)?;

        request_helpers::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_calculate_option_price(request_id, contract, volatility, underlying_price),
            |message| OptionComputation::decode(&self.decoder_context(), message),
        )
        .await
    }

    /// Calculates the implied volatility based on hypothetical option and its underlying prices.
    ///
    /// # Arguments
    /// * `contract`   - The [Contract] object representing the option for which the calculation is being requested.
    /// * `option_price` - Hypothetical option price.
    /// * `underlying_price` - Hypothetical option's underlying price.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::option("AAPL", "20230519", 150.0, OptionRight::Call);
    ///     let calculation = client
    ///         .calculate_implied_volatility(&contract, 25.0, 235.0)
    ///         .await
    ///         .expect("request failed");
    ///     println!("calculation: {calculation:?}");
    /// }
    /// ```
    pub async fn calculate_implied_volatility(
        &self,
        contract: &Contract,
        option_price: f64,
        underlying_price: f64,
    ) -> Result<OptionComputation, Error> {
        check_version(self.server_version(), Features::REQ_CALC_IMPLIED_VOLAT)?;

        request_helpers::one_shot_by_request_id(
            self,
            |request_id| encoders::encode_calculate_implied_volatility(request_id, contract, option_price, underlying_price),
            |message| OptionComputation::decode(&self.decoder_context(), message),
        )
        .await
    }

    /// Cancels an in-flight contract details request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     // `request_id` is the id of the request to cancel, e.g. from `ContractDetailsBuilder::request_id`.
    ///     // Dropping a `contract_details_stream` subscription already cancels it.
    ///     client.cancel_contract_details(42).await.expect("cancel failed");
    /// }
    /// ```
    pub async fn cancel_contract_details(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version(), Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_contract_data(request_id)?;
        self.send_message(message).await?;
        Ok(())
    }

    /// Build a request for an underlying's option chain: one [`OptionChain`] per
    /// exchange the options trade on.
    ///
    /// Terminal: [`OptionChainBuilder::subscribe`]. Optional narrowing via [`OptionChainBuilder::exchange`].
    ///
    /// # Arguments
    /// * `symbol` - Symbol of the underlying.
    /// * `security_type` - Security type of the underlying, e.g. `SecurityType::Stock`.
    /// * `contract_id` - Contract id of the underlying. Required; TWS rejects `0` with
    ///   code 321 "Invalid contract id".
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
    /// }
    /// ```
    pub fn option_chain<'a>(&'a self, symbol: &'a str, security_type: SecurityType, contract_id: i32) -> OptionChainBuilder<'a, Self> {
        OptionChainBuilder::new(self, symbol, security_type, contract_id)
    }
}

/// Send a contract-details request with a pre-allocated id. Reached through
/// [`ContractDetailsBuilder::subscribe`]; the flat arguments are the
/// builder-fed param-budget exception.
pub(in crate::contracts) async fn contract_details_stream(
    client: &Client,
    contract: &Contract,
    request_id: RequestId,
    buffer_limit: Option<usize>,
) -> Result<Subscription<ContractDetails>, Error> {
    let buffer_limit = contract_details_builder::validate_buffer_limit(buffer_limit)?;
    verify::verify_contract(client.server_version(), contract)?;
    let packet = encoders::encode_request_contract_data(request_id.raw(), contract)?;
    let request = client.request_with_id(request_id);
    match buffer_limit {
        Some(limit) => {
            let bound = crate::transport::BufferBound {
                limit,
                end: crate::messages::IncomingMessages::ContractDataEnd,
            };
            request.send_bounded(packet, bound).await
        }
        None => request.send(packet).await,
    }
}

/// Request an underlying's option chain. Reached through
/// [`OptionChainBuilder::subscribe`]; the flat arguments are the builder-fed
/// param-budget exception.
pub(in crate::contracts) async fn option_chain(
    client: &Client,
    symbol: &str,
    exchange: Option<&str>,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<Subscription<OptionChain>, Error> {
    request_helpers::request_with_id(client, Features::SEC_DEF_OPT_PARAMS_REQ, |request_id| {
        encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)
    })
    .await
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
