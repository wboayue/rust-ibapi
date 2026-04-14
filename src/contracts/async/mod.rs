//! Asynchronous implementation of contract management functionality

use super::common::{decoders, encoders, verify};
use super::*;
use crate::client::ClientRequestBuilders;
use crate::common::request_helpers;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{StreamDecoder, Subscription};
use crate::{Client, Error};
use log::{error, info};

impl Client {
    /// Requests contract information.
    ///
    /// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains.
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
        verify::verify_contract(self.server_version(), contract)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let packet = encoders::encode_request_contract_data(request_id, contract)?;

        let mut responses = builder.send_raw(packet).await?;

        let mut contract_details: Vec<ContractDetails> = Vec::default();

        while let Some(response_result) = responses.next().await {
            match response_result {
                Ok(mut response) => {
                    log::debug!("response: {response:#?}");
                    match response.message_type() {
                        IncomingMessages::ContractData => {
                            let decoded = decoders::decode_contract_details(self.server_version(), &mut response)?;
                            contract_details.push(decoded);
                        }
                        IncomingMessages::ContractDataEnd => return Ok(contract_details),
                        IncomingMessages::Error => return Err(Error::from(response)),
                        _ => return Err(Error::UnexpectedResponse(response)),
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::UnexpectedEndOfStream)
    }

    /// Requests matching stock symbols.
    ///
    /// # Arguments
    /// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
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

        let builder = self.request();
        let request_id = builder.request_id();
        let request = encoders::encode_request_matching_symbols(request_id, pattern)?;
        let mut subscription = builder.send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => {
                match message.message_type() {
                    IncomingMessages::SymbolSamples => {
                        return decoders::decode_contract_descriptions(self.server_version(), &mut message);
                    }
                    IncomingMessages::Error => {
                        // TODO custom error
                        error!("unexpected error: {message:?}");
                        return Err(Error::Simple(format!("unexpected error: {message:?}")));
                    }
                    _ => {
                        info!("unexpected message: {message:?}");
                        return Err(Error::Simple(format!("unexpected message: {message:?}")));
                    }
                }
            }
            Some(Err(e)) => return Err(e),
            None => {}
        }

        Ok(Vec::default())
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

        let request = encoders::encode_request_market_rule(market_rule_id)?;
        let mut subscription = self.shared_request(OutgoingMessages::RequestMarketRule).send_raw(request).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => Ok(decoders::decode_market_rule(&mut message)?),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no market rule found".into())),
        }
    }

    /// Calculates an option's price based on the provided volatility and its underlying's price.
    ///
    /// # Arguments
    /// * `contract`   - The [Contract] object for which the depth is being requested.
    /// * `volatility` - Hypothetical volatility.
    /// * `underlying_price` - Hypothetical option's underlying price.
    pub async fn calculate_option_price(&self, contract: &Contract, volatility: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        check_version(self.server_version(), Features::REQ_CALC_OPTION_PRICE)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let message = encoders::encode_calculate_option_price(request_id, contract, volatility, underlying_price)?;
        let mut subscription = builder.send_raw(message).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => OptionComputation::decode(&self.decoder_context(), &mut message),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no data for option calculation".into())),
        }
    }

    /// Calculates the implied volatility based on hypothetical option and its underlying prices.
    ///
    /// # Arguments
    /// * `contract`   - The [Contract] object for which the depth is being requested.
    /// * `option_price` - Hypothetical option price.
    /// * `underlying_price` - Hypothetical option's underlying price.
    pub async fn calculate_implied_volatility(
        &self,
        contract: &Contract,
        option_price: f64,
        underlying_price: f64,
    ) -> Result<OptionComputation, Error> {
        check_version(self.server_version(), Features::REQ_CALC_IMPLIED_VOLAT)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let message = encoders::encode_calculate_implied_volatility(request_id, contract, option_price, underlying_price)?;
        let mut subscription = builder.send_raw(message).await?;

        match subscription.next().await {
            Some(Ok(mut message)) => OptionComputation::decode(&self.decoder_context(), &mut message),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no data for option calculation".into())),
        }
    }

    /// Cancels an in-flight contract details request.
    pub async fn cancel_contract_details(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version(), Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_contract_data(request_id)?;
        self.send_message(message).await?;
        Ok(())
    }

    /// Requests option chain data for an underlying instrument.
    ///
    /// # Arguments
    /// * `symbol` - The underlying symbol
    /// * `exchange` - The exchange
    /// * `security_type` - The underlying security type
    /// * `contract_id` - The underlying contract ID
    pub async fn option_chain(
        &self,
        symbol: &str,
        exchange: &str,
        security_type: SecurityType,
        contract_id: i32,
    ) -> Result<Subscription<OptionChain>, Error> {
        request_helpers::request_with_id(self, Features::SEC_DEF_OPT_PARAMS_REQ, |request_id| {
            encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)
        })
        .await
    }
}

#[cfg(test)]
mod tests;
