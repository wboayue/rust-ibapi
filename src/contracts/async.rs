//! Asynchronous implementation of contract management functionality

use super::common::{decoders, encoders};
use super::*;
use crate::messages::{IncomingMessages, OutgoingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::{AsyncDataStream, Subscription};
use crate::{server_versions, Client, Error};
use log::{error, info};
use std::sync::Arc;

impl AsyncDataStream<OptionComputation> for OptionComputation {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::TickOptionComputation];

    fn decode(client: &Client, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::TickOptionComputation => Ok(decoders::decode_option_computation(client.server_version(), message)?),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        context: &crate::client::builders::ResponseContext,
    ) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("request id required to cancel option calculations");
        match context.request_type {
            Some(OutgoingMessages::ReqCalcImpliedVolat) => {
                encoders::encode_cancel_option_computation(OutgoingMessages::CancelImpliedVolatility, request_id)
            }
            Some(OutgoingMessages::ReqCalcOptionPrice) => encoders::encode_cancel_option_computation(OutgoingMessages::CancelOptionPrice, request_id),
            _ => panic!("Unsupported request message type option computation cancel: {:?}", context.request_type),
        }
    }
}

impl AsyncDataStream<OptionChain> for OptionChain {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::SecurityDefinitionOptionParameter,
        IncomingMessages::SecurityDefinitionOptionParameterEnd,
    ];

    fn decode(_client: &Client, message: &mut ResponseMessage) -> Result<OptionChain, Error> {
        match message.message_type() {
            IncomingMessages::SecurityDefinitionOptionParameter => Ok(decoders::decode_option_chain(message)?),
            IncomingMessages::SecurityDefinitionOptionParameterEnd => Err(Error::EndOfStream),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

/// Requests contract information.
///
/// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
pub async fn contract_details(client: &Client, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
    verify_contract(client, contract).await?;

    let request_id = client.next_request_id();
    let packet = encoders::encode_request_contract_data(client.server_version(), request_id, contract)?;

    let mut responses = client.send_request(request_id, packet).await?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();

    while let Some(mut response) = responses.next().await {
        log::debug!("response: {response:#?}");
        match response.message_type() {
            IncomingMessages::ContractData => {
                let decoded = decoders::decode_contract_details(client.server_version(), &mut response)?;
                contract_details.push(decoded);
            }
            IncomingMessages::ContractDataEnd => return Ok(contract_details),
            IncomingMessages::Error => return Err(Error::from(response)),
            _ => return Err(Error::UnexpectedResponse(response)),
        }
    }

    Err(Error::UnexpectedEndOfStream)
}

pub async fn verify_contract(client: &Client, contract: &Contract) -> Result<(), Error> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(
            server_versions::SEC_ID_TYPE,
            "It does not support security_id_type or security_id attributes",
        )?;
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support the trading_class parameter when requesting contract details.",
        )?;
    }

    if !contract.primary_exchange.is_empty() {
        client.check_server_version(
            server_versions::LINKING,
            "It does not support primary_exchange parameter when requesting contract details.",
        )?;
    }

    if !contract.issuer_id.is_empty() {
        client.check_server_version(
            server_versions::BOND_ISSUERID,
            "It does not support issuer_id parameter when requesting contract details.",
        )?;
    }

    Ok(())
}

/// Requests matching stock symbols.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
pub async fn matching_symbols(client: &Client, pattern: &str) -> Result<Vec<ContractDescription>, Error> {
    client.check_server_version(server_versions::REQ_MATCHING_SYMBOLS, "It does not support matching symbols requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_matching_symbols(request_id, pattern)?;
    let mut subscription = client.send_request(request_id, request).await?;

    if let Some(mut message) = subscription.next().await {
        match message.message_type() {
            IncomingMessages::SymbolSamples => {
                return decoders::decode_contract_descriptions(client.server_version(), &mut message);
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

    Ok(Vec::default())
}

/// Requests details about a given market rule
///
/// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
/// A list of market rule ids can be obtained by invoking [request_contract_details] on a particular contract. The returned market rule ID list will provide the market rule ID for the instrument in the correspond valid exchange list in [ContractDetails].
pub async fn market_rule(client: &Client, market_rule_id: i32) -> Result<MarketRule, Error> {
    client.check_server_version(server_versions::MARKET_RULES, "It does not support market rule requests.")?;

    let request = encoders::encode_request_market_rule(market_rule_id)?;
    let mut subscription = client.send_shared_request(OutgoingMessages::RequestMarketRule, request).await?;

    match subscription.next().await {
        Some(mut message) => Ok(decoders::decode_market_rule(&mut message)?),
        None => Err(Error::Simple("no market rule found".into())),
    }
}

/// Calculates an option's price based on the provided volatility and its underlying's price.
///
/// # Arguments
/// * `contract`   - The [Contract] object for which the depth is being requested.
/// * `volatility` - Hypothetical volatility.
/// * `underlying_price` - Hypothetical option's underlying price.
pub async fn calculate_option_price(
    client: &Client,
    contract: &Contract,
    volatility: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    client.check_server_version(server_versions::REQ_CALC_OPTION_PRICE, "It does not support calculation price requests.")?;

    let request_id = client.next_request_id();
    let message = encoders::encode_calculate_option_price(client.server_version(), request_id, contract, volatility, underlying_price)?;
    let mut subscription = client.send_request(request_id, message).await?;

    match subscription.next().await {
        Some(mut message) => OptionComputation::decode(client, &mut message),
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
    client: &Client,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    client.check_server_version(
        server_versions::REQ_CALC_IMPLIED_VOLAT,
        "It does not support calculate implied volatility.",
    )?;

    let request_id = client.next_request_id();
    let message = encoders::encode_calculate_implied_volatility(client.server_version(), request_id, contract, option_price, underlying_price)?;
    let mut subscription = client.send_request(request_id, message).await?;

    match subscription.next().await {
        Some(mut message) => OptionComputation::decode(client, &mut message),
        None => Err(Error::Simple("no data for option calculation".into())),
    }
}

pub async fn option_chain(
    client: &Client,
    symbol: &str,
    exchange: &str,
    security_type: SecurityType,
    contract_id: i32,
) -> Result<Subscription<OptionChain>, Error> {
    client.check_server_version(
        server_versions::SEC_DEF_OPT_PARAMS_REQ,
        "It does not support security definition option parameters.",
    )?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)?;
    let internal_subscription = client.send_request(request_id, request).await?;

    Ok(Subscription::new_from_internal_simple::<OptionChain>(
        internal_subscription,
        Arc::new(client.clone()),
    ))
}
