use super::common::{decoders, encoders};
use super::*;
use crate::client::blocking::{ClientRequestBuilders, Subscription};
use crate::common::request_helpers;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::protocol::{check_version, Features};
use crate::subscriptions::StreamDecoder;
use crate::{client::sync::Client, Error};
use log::{error, info};

impl Client {
    fn verify_contract(&self, contract: &Contract) -> Result<(), Error> {
        if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
            check_version(self.server_version, Features::SEC_ID_TYPE)?
        }

        if !contract.trading_class.is_empty() {
            check_version(self.server_version, Features::TRADING_CLASS)?
        }

        if !contract.primary_exchange.is_empty() {
            check_version(self.server_version, Features::LINKING)?
        }

        if !contract.issuer_id.is_empty() {
            check_version(self.server_version, Features::BOND_ISSUERID)?
        }

        Ok(())
    }

    /// Requests contract information.
    ///
    /// Provides all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use [Client::option_chain] for that purpose.
    ///
    /// # Arguments
    /// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::stock("TSLA").build();
    /// let results = client.contract_details(&contract).expect("request failed");
    /// for contract_detail in results {
    ///     println!("contract: {contract_detail:?}");
    /// }
    /// ```
    pub fn contract_details(&self, contract: &Contract) -> Result<Vec<ContractDetails>, Error> {
        self.verify_contract(contract)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let packet = encoders::encode_request_contract_data(self.server_version, request_id, contract)?;

        let responses = builder.send_raw(packet)?;

        let mut contract_details: Vec<ContractDetails> = Vec::default();

        while let Some(response) = responses.next() {
            log::debug!("response: {response:#?}");
            match response {
                Ok(mut message) if message.message_type() == IncomingMessages::ContractData => {
                    let decoded = decoders::decode_contract_details(self.server_version, &mut message)?;
                    contract_details.push(decoded);
                }
                Ok(message) if message.message_type() == IncomingMessages::ContractDataEnd => return Ok(contract_details),
                Ok(message) if message.message_type() == IncomingMessages::Error => return Err(Error::from(message)),
                Ok(message) => return Err(Error::UnexpectedResponse(message)),
                Err(e) => return Err(e),
            }
        }

        Err(Error::UnexpectedEndOfStream)
    }

    /// Cancels an in-flight contract details request.
    ///
    /// # Arguments
    /// * `request_id` - The request ID returned by a prior `contract_details` call.
    pub fn cancel_contract_details(&self, request_id: i32) -> Result<(), Error> {
        check_version(self.server_version, Features::CANCEL_CONTRACT_DATA)?;

        let message = encoders::encode_cancel_contract_data(request_id)?;
        self.send_message(message)?;
        Ok(())
    }

    /// Requests details about a given market rule
    ///
    /// The market rule for an instrument on a particular exchange provides details about how the minimum price increment changes with price.
    /// A list of market rule ids can be obtained by invoking [Self::contract_details()] for a particular contract.
    /// The returned market rule ID list will provide the market rule ID for the instrument in the correspond valid exchange list in [contracts::ContractDetails].
    pub fn market_rule(&self, market_rule_id: i32) -> Result<MarketRule, Error> {
        check_version(self.server_version, Features::MARKET_RULES)?;

        let request = encoders::encode_request_market_rule(market_rule_id)?;
        let subscription = self.shared_request(OutgoingMessages::RequestMarketRule).send_raw(request)?;

        match subscription.next() {
            Some(Ok(mut message)) => Ok(decoders::decode_market_rule(&mut message)?),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no market rule found".into())),
        }
    }

    /// Requests matching stock symbols.
    ///
    /// # Arguments
    /// * `pattern` - Either start of ticker symbol or (for larger strings) company name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contracts = client.matching_symbols("IB").expect("request failed");
    /// for contract in contracts {
    ///     println!("contract: {contract:?}");
    /// }
    /// ```
    pub fn matching_symbols(&self, pattern: &str) -> Result<impl Iterator<Item = ContractDescription>, Error> {
        check_version(self.server_version, Features::REQ_MATCHING_SYMBOLS)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let request = encoders::encode_request_matching_symbols(request_id, pattern)?;
        let subscription = builder.send_raw(request)?;

        if let Some(Ok(mut message)) = subscription.next() {
            match message.message_type() {
                IncomingMessages::SymbolSamples => {
                    return Ok(decoders::decode_contract_descriptions(self.server_version, &mut message)?.into_iter());
                }
                IncomingMessages::Error => {
                    error!("unexpected error: {message:?}");
                    return Err(Error::Simple(format!("unexpected error: {message:?}")));
                }
                _ => {
                    info!("unexpected message: {message:?}");
                    return Err(Error::Simple(format!("unexpected message: {message:?}")));
                }
            }
        }

        Ok(Vec::default().into_iter())
    }

    /// Calculates an option's price based on the provided volatility and its underlying's price.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] object representing the option for which the calculation is being requested.
    /// * `volatility`      - Hypothetical volatility as a percentage (e.g., 20.0 for 20%).
    /// * `underlying_price` - Hypothetical price of the underlying asset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::option("AAPL", "20251219", 150.0, "C");
    /// let calculation = client.calculate_option_price(&contract, 100.0, 235.0).expect("request failed");
    /// println!("calculation: {calculation:?}");
    /// ```
    pub fn calculate_option_price(&self, contract: &Contract, volatility: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        check_version(self.server_version, Features::REQ_CALC_OPTION_PRICE)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let message = encoders::encode_calculate_option_price(self.server_version, request_id, contract, volatility, underlying_price)?;
        let subscription = builder.send_raw(message)?;

        match subscription.next() {
            Some(Ok(mut message)) => OptionComputation::decode(&self.decoder_context(), &mut message),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no data for option calculation".into())),
        }
    }

    /// Calculates the implied volatility based on the hypothetical option price and underlying price.
    ///
    /// # Arguments
    /// * `contract`        - The [Contract] object representing the option for which the calculation is being requested.
    /// * `option_price`    - Hypothetical option price.
    /// * `underlying_price` - Hypothetical price of the underlying asset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let contract = Contract::option("AAPL", "20230519", 150.0, "C");
    /// let calculation = client.calculate_implied_volatility(&contract, 25.0, 235.0).expect("request failed");
    /// println!("calculation: {calculation:?}");
    /// ```
    pub fn calculate_implied_volatility(&self, contract: &Contract, option_price: f64, underlying_price: f64) -> Result<OptionComputation, Error> {
        check_version(self.server_version, Features::REQ_CALC_IMPLIED_VOLAT)?;

        let builder = self.request();
        let request_id = builder.request_id();
        let message = encoders::encode_calculate_implied_volatility(self.server_version, request_id, contract, option_price, underlying_price)?;
        let subscription = builder.send_raw(message)?;

        match subscription.next() {
            Some(Ok(mut message)) => OptionComputation::decode(&self.decoder_context(), &mut message),
            Some(Err(e)) => Err(e),
            None => Err(Error::Simple("no data for option calculation".into())),
        }
    }

    /// Requests security definition option parameters for viewing a contract's option chain.
    ///
    /// # Arguments
    /// `symbol`   - Contract symbol of the underlying.
    /// `exchange` - The exchange on which the returned options are trading. Can be set to the empty string for all exchanges.
    /// `security_type` - The type of the underlying security, i.e. STK
    /// `contract_id`   - The contract ID of the underlying security.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::SecurityType;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let symbol = "AAPL";
    /// let exchange = ""; // all exchanges
    /// let security_type = SecurityType::Stock;
    /// let contract_id = 265598;
    ///
    /// let subscription = client
    ///     .option_chain(symbol, exchange, security_type, contract_id)
    ///     .expect("request option chain failed!");
    ///
    /// for option_chain in &subscription {
    ///     println!("{option_chain:?}")
    /// }
    /// ```
    pub fn option_chain(
        &self,
        symbol: &str,
        exchange: &str,
        security_type: SecurityType,
        contract_id: i32,
    ) -> Result<Subscription<OptionChain>, Error> {
        request_helpers::blocking::request_with_id(self, Features::SEC_DEF_OPT_PARAMS_REQ, |request_id| {
            encoders::encode_request_option_chain(request_id, symbol, exchange, security_type, contract_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::common::test_tables::*;
    use crate::messages::ResponseMessage;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use crate::subscriptions::{DecoderContext, StreamDecoder};
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_contract_details() {
        for test_case in contract_details_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
            let result = client.contract_details(&test_case.contract);

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

    #[test]
    fn test_matching_symbols() {
        for test_case in matching_symbols_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID);
            let result = client.matching_symbols(test_case.pattern);

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let symbols: Vec<_> = result.unwrap().collect();
            assert_eq!(symbols.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
        }
    }

    #[test]
    fn test_market_rule() {
        for test_case in market_rule_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::MARKET_RULES);
            let result = client.market_rule(test_case.market_rule_id);

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

    #[test]
    fn test_option_calculations() {
        for test_case in option_calculation_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![test_case.response_message.clone()],
            });

            let client = Client::stubbed(message_bus, server_versions::REQ_CALC_OPTION_PRICE);

            let result = if let Some(volatility) = test_case.volatility {
                client.calculate_option_price(&test_case.contract, volatility, test_case.underlying_price)
            } else if let Some(option_price) = test_case.option_price {
                client.calculate_implied_volatility(&test_case.contract, option_price, test_case.underlying_price)
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

    #[test]
    fn test_option_chain() {
        for test_case in option_chain_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SEC_DEF_OPT_PARAMS_REQ);
            let result = client.option_chain(
                test_case.symbol,
                test_case.exchange,
                test_case.security_type.clone(),
                test_case.contract_id,
            );

            let request_messages = client.message_bus.request_messages();
            assert_eq!(
                request_messages[0].encode_simple(),
                test_case.expected_request,
                "Test '{}' request mismatch",
                test_case.name
            );

            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
            let subscription = result.unwrap();

            let chains: Vec<_> = subscription.into_iter().collect();
            assert_eq!(chains.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
        }
    }

    #[test]
    fn test_verify_contract() {
        for test_case in verify_contract_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: vec![],
            });

            let client = Client::stubbed(message_bus, test_case.server_version);
            let result = client.verify_contract(&test_case.contract);

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

    #[test]
    fn test_stream_decoders() {
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

    #[test]
    fn test_cancel_messages() {
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

    #[test]
    fn test_client_methods() {
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
                } => client.calculate_option_price(contract, *volatility, *underlying_price),
                ClientMethodTest::CalculateImpliedVolatility {
                    contract,
                    option_price,
                    underlying_price,
                } => client.calculate_implied_volatility(contract, *option_price, *underlying_price),
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

    #[test]
    fn test_contract_details_errors() {
        for test_case in contract_details_error_test_cases() {
            let message_bus = Arc::new(MessageBusStub {
                request_messages: RwLock::new(vec![]),
                response_messages: test_case.response_messages.clone(),
            });

            let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
            let result = client.contract_details(&test_case.contract);

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
