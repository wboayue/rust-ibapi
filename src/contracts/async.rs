//! Asynchronous implementation of contract management functionality

use super::common::{decoders, encoders};
use super::*;
use crate::client::ClientRequestBuilders;
use crate::common::request_helpers;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::protocol::{check_version, Features};
use crate::subscriptions::{StreamDecoder, Subscription};
use crate::{Client, Error};
use log::{error, info};

/// Requests contract information.
///
/// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
pub async fn contract_details(client: &Client, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
    verify_contract(client, contract).await?;

    let builder = client.request();
    let request_id = builder.request_id();
    let packet = encoders::encode_request_contract_data(client.server_version(), request_id, contract)?;

    let mut responses = builder.send_raw(packet).await?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();

    while let Some(response_result) = responses.next().await {
        match response_result {
            Ok(mut response) => {
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
            Err(e) => return Err(e),
        }
    }

    Err(Error::UnexpectedEndOfStream)
}

pub async fn verify_contract(client: &Client, contract: &Contract) -> Result<(), Error> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        check_version(client.server_version(), Features::SEC_ID_TYPE)?;
    }

    if !contract.trading_class.is_empty() {
        check_version(client.server_version(), Features::TRADING_CLASS)?;
    }

    if !contract.primary_exchange.is_empty() {
        check_version(client.server_version(), Features::LINKING)?;
    }

    if !contract.issuer_id.is_empty() {
        check_version(client.server_version(), Features::BOND_ISSUERID)?;
    }

    Ok(())
}

/// Requests matching stock symbols.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
pub async fn matching_symbols(client: &Client, pattern: &str) -> Result<Vec<ContractDescription>, Error> {
    check_version(client.server_version(), Features::REQ_MATCHING_SYMBOLS)?;

    let builder = client.request();
    let request_id = builder.request_id();
    let request = encoders::encode_request_matching_symbols(request_id, pattern)?;
    let mut subscription = builder.send_raw(request).await?;

    match subscription.next().await {
        Some(Ok(mut message)) => {
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
        Some(Err(e)) => return Err(e),
        None => {}
    }

    Ok(Vec::default())
}

/// Requests details about a given market rule
///
/// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
/// A list of market rule ids can be obtained by invoking [request_contract_details] on a particular contract. The returned market rule ID list will provide the market rule ID for the instrument in the correspond valid exchange list in [ContractDetails].
pub async fn market_rule(client: &Client, market_rule_id: i32) -> Result<MarketRule, Error> {
    check_version(client.server_version(), Features::MARKET_RULES)?;

    let request = encoders::encode_request_market_rule(market_rule_id)?;
    let mut subscription = client.shared_request(OutgoingMessages::RequestMarketRule).send_raw(request).await?;

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
pub async fn calculate_option_price(
    client: &Client,
    contract: &Contract,
    volatility: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    check_version(client.server_version(), Features::REQ_CALC_OPTION_PRICE)?;

    let builder = client.request();
    let request_id = builder.request_id();
    let message = encoders::encode_calculate_option_price(client.server_version(), request_id, contract, volatility, underlying_price)?;
    let mut subscription = builder.send_raw(message).await?;

    match subscription.next().await {
        Some(Ok(mut message)) => OptionComputation::decode(&client.decoder_context(), &mut message),
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
    client: &Client,
    contract: &Contract,
    option_price: f64,
    underlying_price: f64,
) -> Result<OptionComputation, Error> {
    check_version(client.server_version(), Features::REQ_CALC_IMPLIED_VOLAT)?;

    let builder = client.request();
    let request_id = builder.request_id();
    let message = encoders::encode_calculate_implied_volatility(client.server_version(), request_id, contract, option_price, underlying_price)?;
    let mut subscription = builder.send_raw(message).await?;

    match subscription.next().await {
        Some(Ok(mut message)) => OptionComputation::decode(&client.decoder_context(), &mut message),
        Some(Err(e)) => Err(e),
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
    request_helpers::request_with_id(client, Features::SEC_DEF_OPT_PARAMS_REQ, |request_id| {
        encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::common::test_tables::*;
    use crate::contracts::{Currency, Exchange, Symbol};
    use crate::messages::ResponseMessage;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::subscriptions::{DecoderContext, StreamDecoder};
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn test_contract_details() {
        for test_case in contract_details_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
            let result = contract_details(&client, &test_case.contract).await;

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let contracts = result.unwrap();
            assert_eq!(contracts.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);

            (test_case.validations)(&contracts);
        }
    }

    #[tokio::test]
    async fn test_matching_symbols() {
        for test_case in matching_symbols_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID);
            let result = matching_symbols(&client, test_case.pattern).await;

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let symbols = result.unwrap();
            assert_eq!(symbols.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
        }
    }

    #[tokio::test]
    async fn test_market_rule() {
        for test_case in market_rule_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::MARKET_RULES);
            let result = market_rule(&client, test_case.market_rule_id).await;

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let rule = result.unwrap();
            assert_eq!(
                rule.price_increments.len(),
                test_case.expected_price_increments,
                "Test '{}' price increments mismatch",
                test_case.name
            );
        }
    }

    #[tokio::test]
    async fn test_option_calculations() {
        for test_case in option_calculation_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::REQ_CALC_OPTION_PRICE);

            let result = if let Some(volatility) = test_case.volatility {
                calculate_option_price(&client, &test_case.contract, volatility, test_case.underlying_price).await
            } else if let Some(option_price) = test_case.option_price {
                calculate_implied_volatility(&client, &test_case.contract, option_price, test_case.underlying_price).await
            } else {
                panic!("Test case must have either volatility or option_price");
            };

            let request_messages = client.message_bus.request_messages();
            assert!(
                request_messages[0].encode_simple().starts_with(test_case.expected_request_prefix),
                "Test '{}' request mismatch: expected prefix '{}', got '{}'",
                test_case.name,
                test_case.expected_request_prefix,
                request_messages[0].encode_simple()
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let computation = result.unwrap();
            assert_eq!(
                computation.option_price,
                Some(test_case.expected_price),
                "Test '{}' price mismatch",
                test_case.name
            );
            assert_eq!(
                computation.delta,
                Some(test_case.expected_delta),
                "Test '{}' delta mismatch",
                test_case.name
            );
        }
    }

    #[tokio::test]
    async fn test_option_chain() {
        for test_case in option_chain_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SEC_DEF_OPT_PARAMS_REQ);
            let result = option_chain(
                &client,
                test_case.symbol,
                test_case.exchange,
                test_case.security_type.clone(),
                test_case.contract_id,
            )
            .await;

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let mut subscription = result.unwrap();

            let mut count = 0;
            while subscription.next().await.is_some() {
                count += 1;
            }
            assert_eq!(count, test_case.expected_count, "Test '{}' count mismatch", test_case.name);
        }
    }

    #[tokio::test]
    async fn test_verify_contract() {
        for test_case in verify_contract_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![],
            });

            let client = Client::stubbed(message_bus, test_case.server_version);
            let result = verify_contract(&client, &test_case.contract).await;

            if test_case.should_error {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                if let Some(expected_error) = test_case.error_contains {
                    let error_msg = format!("{:?}", result.err());
                    assert!(
                        error_msg.contains(expected_error),
                        "Test '{}' error should contain '{}', got '{}'",
                        test_case.name,
                        expected_error,
                        error_msg
                    );
                }
            } else {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            }
        }
    }

    #[tokio::test]
    async fn test_stream_decoders() {
        for test_case in stream_decoder_test_cases() {
            let mut message = ResponseMessage::from(test_case.message);

            match &test_case.expected_result {
                StreamDecoderResult::OptionComputation { price, delta } => {
                    let result = OptionComputation::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                    let computation = result.unwrap();
                    assert_eq!(computation.option_price, Some(*price), "Test '{}' price mismatch", test_case.name);
                    assert_eq!(computation.delta, Some(*delta), "Test '{}' delta mismatch", test_case.name);
                }
                StreamDecoderResult::OptionChain { exchange, underlying_conid } => {
                    let result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                    let chain = result.unwrap();
                    assert_eq!(chain.exchange, *exchange, "Test '{}' exchange mismatch", test_case.name);
                    assert_eq!(
                        chain.underlying_contract_id, *underlying_conid,
                        "Test '{}' conid mismatch",
                        test_case.name
                    );
                }
                StreamDecoderResult::Error(expected_error) => {
                    match test_case.message {
                        msg if msg.starts_with("76") => {
                            // OptionChain end of stream
                            let result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                            assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                            assert!(
                                format!("{:?}", result.err()).contains(expected_error),
                                "Test '{}' wrong error",
                                test_case.name
                            );
                        }
                        _ => {
                            // Try both decoders
                            let opt_result = OptionComputation::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message.clone());
                            let chain_result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                            assert!(
                                opt_result.is_err() && chain_result.is_err(),
                                "Test '{}' should have failed",
                                test_case.name
                            );
                        }
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_cancel_messages() {
        for test_case in cancel_message_test_cases() {
            let context = test_case.request_type.map(|rt| DecoderContext::default().with_request_type(rt));

            let result = match test_case.decoder_type {
                "OptionComputation" => OptionComputation::cancel_message(server_versions::SIZE_RULES, test_case.request_id, context.as_ref()),
                "OptionChain" => {
                    // OptionChain doesn't implement cancel_message
                    Err(crate::Error::Simple("cancel not implemented".to_string()))
                }
                _ => panic!("Unknown decoder type"),
            };

            match &test_case.expected_message {
                Ok(expected) => {
                    assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                    assert_eq!(result.unwrap().encode_simple(), *expected, "Test '{}' message mismatch", test_case.name);
                }
                Err(expected_error) => {
                    assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                    let error_msg = format!("{:?}", result.err());
                    assert!(
                        error_msg.contains(expected_error),
                        "Test '{}' error should contain '{}', got '{}'",
                        test_case.name,
                        expected_error,
                        error_msg
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn request_stock_contract_details() {
        let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "10|9001|TSLA|STK||0||SMART|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0400-20221229:2000;20221230:0400-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0400-20230103:2000|20221229:0930-20221229:1600;20221230:0930-20221230:1600;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0930-20230103:1600|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "10|9001|TSLA|STK||0||AMEX|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALLOC,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,IOC,LIT,LMT,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "52|1|9001||".to_string(),
        ]
    });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        let contract = Contract::stock("TSLA").build();

        let results = client.contract_details(&contract).await;

        let request_messages = client.message_bus.request_messages();

        assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||");

        assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

        let contracts: Vec<ContractDetails> = results.unwrap();
        assert_eq!(2, contracts.len());

        assert_eq!(contracts[0].contract.exchange, Exchange::from("SMART"));
        assert_eq!(contracts[1].contract.exchange, Exchange::from("AMEX"));

        assert_eq!(contracts[0].contract.symbol, Symbol::from("TSLA"));
        assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
        assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
        assert_eq!(contracts[0].contract.contract_id, 76792991);
        assert_eq!(contracts[0].contract.primary_exchange, Exchange::from("NASDAQ"));
        assert_eq!(contracts[0].long_name, "TESLA INC");
        assert_eq!(contracts[0].stock_type, "COMMON");
        assert_eq!(contracts[0].min_size, 1.0);
        assert_eq!(contracts[0].size_increment, 1.0);
        assert_eq!(contracts[0].suggested_size_increment, 100.0);
    }

    #[tokio::test]
    #[ignore = "reason: need sample messages"]
    async fn request_bond_contract_details() {
        let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Format similar to request_stock_contract_details but with bond-specific fields
            "10|9001|TLT|BOND|20420815|0||||USD|TLT|US Treasury Bond|BOND|12345|0.01|1000|SMART|NYSE|SMART|NYSE|1|0|US Treasury Bond|SMART||Government||US/Eastern|20221229:0400-20221229:2000;20221230:0400-20221230:2000|20221229:0930-20221229:1600;20221230:0930-20221230:1600|||1|CUSIP|912810TL8|1|||26|20420815|GOVT|1|1|2.25|0|20420815|20120815|20320815|CALL|100.0|1|Government Bond Notes|0.1|0.01|1|".to_string(),
            "52|1|9001||".to_string(),
        ]
    });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        // Create a bond contract
        let contract = Contract {
            symbol: Symbol::from("TLT"),
            security_type: SecurityType::Bond,
            exchange: Exchange::from("SMART"),
            currency: Currency::from("USD"),
            ..Default::default()
        };

        let results = client.contract_details(&contract).await;

        let request_messages = client.message_bus.request_messages();

        // Check if the request was encoded correctly
        assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|TLT|BOND||0|||SMART||USD|||0|||");

        assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

        let contracts: Vec<ContractDetails> = results.unwrap();
        assert_eq!(1, contracts.len());

        // Check basic contract fields
        assert_eq!(contracts[0].contract.symbol, Symbol::from("TLT"));
        assert_eq!(contracts[0].contract.security_type, SecurityType::Bond);
        assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
        assert_eq!(contracts[0].contract.contract_id, 12345);

        // Check bond-specific fields
        assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20420815");
        assert_eq!(contracts[0].cusip, "912810TL8");
        assert_eq!(contracts[0].coupon, 2.25);
        assert_eq!(contracts[0].maturity, "20420815");
        assert_eq!(contracts[0].issue_date, "20120815");
        assert_eq!(contracts[0].next_option_date, "20320815");
        assert_eq!(contracts[0].next_option_type, "CALL");
        assert_eq!(contracts[0].next_option_partial, true);
        assert_eq!(contracts[0].notes, "Government Bond Notes");
    }

    #[tokio::test]
    async fn request_future_contract_details() {
        let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "10|9000|ES|FUT|20250620 08:30 US/Central|0||CME|USD|ESM5|ES|ES|620731015|0.25|50|ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AVGCOST,BASKET,BENCHPX,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,ICE,IOC,LIT,LMT,LTH,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|CME,QBALGO|1|11004968|E-mini S&P 500||202506||||US/Central|20250521:1700-20250522:1600;20250522:1700-20250523:1600;20250524:CLOSED;20250525:1700-20250526:1200;20250526:1700-20250527:1600;20250527:1700-20250528:1600|20250522:0830-20250522:1600;20250523:0830-20250523:1600;20250524:CLOSED;20250525:1700-20250526:1200;20250527:0830-20250527:1600;20250527:1700-20250528:1600|||0|2147483647|ES|IND|67,67|20250620||1|1|1|".to_string(),
            "52|1|9000|".to_string(),
        ]
    });

        let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

        // Create a future contract
        let contract = Contract {
            symbol: Symbol::from("ES"),
            security_type: SecurityType::Future,
            last_trade_date_or_contract_month: "202506".to_string(),
            exchange: Exchange::from("GLOBEX"),
            currency: Currency::from("USD"),
            ..Default::default()
        };

        let results = client.contract_details(&contract).await;

        let request_messages = client.message_bus.request_messages();

        // Check if the request was encoded correctly
        assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|ES|FUT|202506|0|||GLOBEX||USD|||0|||");

        assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

        let contracts: Vec<ContractDetails> = results.unwrap();
        assert_eq!(1, contracts.len());

        // Check basic contract fields
        assert_eq!(contracts[0].contract.symbol, Symbol::from("ES"));
        assert_eq!(contracts[0].contract.security_type, SecurityType::Future);
        assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
        assert_eq!(contracts[0].contract.contract_id, 620731015);

        // Check future-specific fields
        assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20250620");
        assert_eq!(contracts[0].contract.multiplier, "50");
        assert_eq!(contracts[0].contract.local_symbol, "ESM5");
        assert_eq!(contracts[0].contract.trading_class, "ES");
        assert_eq!(contracts[0].contract.exchange, Exchange::from("CME"));
        assert_eq!(contracts[0].min_tick, 0.25);
        assert_eq!(contracts[0].market_name, "ES");
        assert_eq!(contracts[0].contract_month, "202506");
        assert_eq!(contracts[0].real_expiration_date, "20250620");
    }

    #[tokio::test]
    async fn test_client_methods() {
        for test_case in client_method_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(
                message_bus,
                match &test_case.test_type {
                    ClientMethodTest::CalculateOptionPrice { .. } => server_versions::REQ_CALC_OPTION_PRICE,
                    ClientMethodTest::CalculateImpliedVolatility { .. } => server_versions::REQ_CALC_IMPLIED_VOLAT,
                },
            );

            let result = match &test_case.test_type {
                ClientMethodTest::CalculateOptionPrice {
                    contract,
                    volatility,
                    underlying_price,
                } => client.calculate_option_price(contract, *volatility, *underlying_price).await,
                ClientMethodTest::CalculateImpliedVolatility {
                    contract,
                    option_price,
                    underlying_price,
                } => client.calculate_implied_volatility(contract, *option_price, *underlying_price).await,
            };

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            let computation = result.unwrap();
            match &test_case.expected_result {
                ClientMethodResult::OptionComputation {
                    option_price,
                    implied_volatility,
                } => {
                    assert_eq!(computation.option_price, *option_price, "Test '{}' option price mismatch", test_case.name);
                    assert_eq!(
                        computation.implied_volatility, *implied_volatility,
                        "Test '{}' implied volatility mismatch",
                        test_case.name
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_contract_details_errors() {
        for test_case in contract_details_error_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
            let result = client.contract_details(&test_case.contract).await;

            if test_case.should_error {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                if let Some(expected_error) = test_case.error_contains {
                    let error_msg = result.unwrap_err().to_string();
                    assert!(
                        error_msg.contains(expected_error),
                        "Test '{}' error should contain '{}', got '{}'",
                        test_case.name,
                        expected_error,
                        error_msg
                    );
                }
            } else {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                let contracts = result.unwrap();
                assert_eq!(contracts.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
            }
        }
    }
}
