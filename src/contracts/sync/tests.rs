use super::*;
use crate::common::test_utils::helpers::assert_proto_msg_id;
use crate::contracts::common::test_tables::*;
use crate::messages::{OutgoingMessages, ResponseMessage};
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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestContractData);

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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMatchingSymbols);

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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestMarketRule);

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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);

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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);
        assert_proto_msg_id(&request_messages[0], OutgoingMessages::RequestSecurityDefinitionOptionalParameters);

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
        let result = verify::verify_contract(client.server_version, &test_case.contract);

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

        match &test_case.expected_msg_id {
            Ok(expected_id) => {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                let bytes = result.unwrap();
                let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                assert_eq!(msg_id, *expected_id, "Test '{}' message id mismatch", test_case.name);
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
        assert!(!request_messages.is_empty(), "Test '{}' should have sent a request", test_case.name);

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
